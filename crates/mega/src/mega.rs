// This crate delegate mega and its fuse daemon.
// The following requirements should be met:
//
// TODO:
// 1. Only one daemon on this machine.
//      This should be both warrantied by this module and scorpio
// 2. At least one daemon on this machine when zed startup.
// 3. Complete docs.
// 4. Add settings for this module

use crate::api::{
    ConfigRequest, ConfigResponse, MountRequest, MountResponse, MountsResponse, UmountRequest,
    UmountResponse,
};
use crate::mega_settings::MegaSettings;
use crate::Event::FuseMounted;
use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;
use futures::{AsyncReadExt, FutureExt, SinkExt, TryFutureExt};
use gpui::http_client::{AsyncBody, HttpClient, HttpRequestExt};
use gpui::{AppContext, Context, EventEmitter, ModelContext, Path};
use radix_trie::{Trie, TrieCommon};
use reqwest_client::ReqwestClient;
use schemars::_private::NoSerialize;
use serde::Serialize;
use settings::Settings;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

mod api;
mod mega_settings;

pub fn init(cx: &mut AppContext) {
    Mega::init(cx);
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    FuseRunning(bool),
    FuseMounted(Option<PathBuf>),
    FuseCheckout(Option<PathBuf>),
}

struct CheckoutState {
    path: PathBuf,
    mounted: bool,
}
pub struct Mega {
    fuse_running: bool,
    fuse_mounted: bool,

    mount_point: Option<PathBuf>,
    checkout_path: Trie<String, u64>,

    mega_url: String,
    fuse_url: String,
    http_client: Arc<ReqwestClient>,
}

pub struct MegaFuse {}

impl EventEmitter<Event> for Mega {}

impl Mega {
    pub fn init_settings(cx: &mut AppContext) {
        MegaSettings::register(cx);
    }

    pub fn init(cx: &mut AppContext) {
        Self::init_settings(cx);
    }

    pub fn new(cx: &mut AppContext) -> Self {
        let mount_path = PathBuf::from(MegaSettings::get_global(cx).mount_point.clone());
        let mega_url = MegaSettings::get_global(cx).mega_url.clone();
        let fuse_url = MegaSettings::get_global(cx).fuse_url.clone();

        // To not affected by global proxy settings.
        let client = ReqwestClient::new();

        let mount_point = if mount_path.exists() {
            Some(mount_path)
        } else {
            None
        };

        Mega {
            fuse_running: false,
            fuse_mounted: false,

            mount_point,
            checkout_path: Default::default(),

            mega_url,
            fuse_url,
            http_client: Arc::new(client),
        }
    }

    pub fn update_status(&mut self, cx: &mut ModelContext<Self>) {
        let checkouts = self.get_checkout_paths(cx);
        let config = self.get_fuse_config(cx);

        cx.spawn(|this, mut cx| async move {
            if let Ok(opt) = checkouts.await {
                match opt {
                    None => {
                        // This means we cannot connect to a localhost port.
                        // So we can assume that fuse has been dead.
                        this.update(&mut cx, |mega, cx| {
                            mega.fuse_running = false;
                            cx.emit(Event::FuseRunning(false));
                            cx.emit(Event::FuseMounted(None));
                        })
                    }
                    Some(info) => {
                        // Check if checkout-ed paths are correct
                        this.update(&mut cx, |mega, cx| {
                            mega.fuse_running = true;
                            
                            let trie = &mut mega.checkout_path;
                            for ref i in info.mounts {
                                let missing = trie.get_ancestor(&i.path).is_none();
                                if missing {
                                    // Should not happen unless on startup.
                                    trie.insert(i.path.clone(), i.inode);
                                    cx.emit(Event::FuseCheckout(Some(PathBuf::from(i.path.clone()))))
                                }
                            }
                        })
                    }
                }
            } else { Ok(()) }.unwrap();

            // When mount point changed, emit an event.
            if let Ok(Some(config)) = config.await {
                this.update(&mut cx, |this, cx| {
                    let path = PathBuf::from(config.config.mount_path);
                    if (this.fuse_mounted && this.fuse_running)
                    && this.mount_point.is_some() {
                        if let Some(inner) = &this.mount_point {
                            if !inner.eq(&path) {
                                this.mount_point = Some(path);
                                cx.emit(Event::FuseMounted(this.mount_point.clone()));
                            }
                        }
                    }
                })
            } else { Ok(()) }
        })
        .detach();
    }

    pub fn status(&self) -> (bool, bool) {
        (self.fuse_running, self.fuse_mounted)
    }

    pub fn toggle_fuse(&mut self, cx: &mut ModelContext<Self>) {
        self.update_status(cx);
        let paths = &self.checkout_path;

        if !self.fuse_mounted {
            for (_, (p, _)) in paths.iter().enumerate() {
                let path = PathBuf::from(p); // FIXME is there a better way?
                cx.spawn(|mega, mut cx| async move {
                    let recv = mega.update(&mut cx, |this, cx| {
                        let param = PathBuf::from(path);
                        this.checkout_path(cx, param)
                    }).expect("mega delegate not be dropped");

                    if let Ok(Some(resp)) = recv.await {
                        mega.update(&mut cx, |this, cx| {
                            let buf = PathBuf::from(resp.mount.path.clone());
                            cx.emit(Event::FuseCheckout(Some(buf)));
                        })
                    } else {
                        Ok(())
                    }
                })
                    .detach();
            }

            self.fuse_mounted = true;
            // FIXME: A configurable path from fuse api is needed.
            cx.emit(Event::FuseMounted(Some(PathBuf::from("/home/neon/dic"))));
        } else {
            for (_, (p, &n)) in paths.iter().enumerate() {
                let path = PathBuf::from(p); // FIXME is there a better way?
                cx.spawn(|mega, mut cx| async move {
                    let recv = mega.update(&mut cx, |this, cx| {
                        let param = PathBuf::from(path);
                        this.restore_path(cx, param, n)
                    }).expect("mega delegate not be dropped");

                    if let Ok(Some(resp)) = recv.await {
                        mega.update(&mut cx, |this, cx| {
                            // TODO use a new check out state struct
                            cx.emit(Event::FuseCheckout(None));
                        })
                    } else {
                        Ok(())
                    }
                })
                    .detach();
            }

            self.fuse_mounted = false;
            cx.emit(Event::FuseMounted(None));
        }
    }

    pub fn toggle_mount(&mut self, cx: &mut ModelContext<Self>) {
        // FIXME should be able to restart fuse
        self.fuse_running = !self.fuse_running;
        cx.emit(Event::FuseRunning(self.fuse_running));
    }

    pub fn checkout_path(
        &self,
        cx: &ModelContext<Self>,
        path: PathBuf,
    ) -> Receiver<Option<MountResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!(
            // FIXME: settings not work, currently
            "{base}/api/fs/mount",
            base = self.fuse_url
        );

        // If it panics, that means there's a bug in code.
        let path = path.to_str().unwrap();
        let req = MountRequest { path };
        let body = serde_json::to_string(&req).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client
                .post_json(
                    "http://127.0.0.1:2725/api/fs/mount",
                    AsyncBody::from(body),
                )
                .await
            {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<MountResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn checkout_multi_path(&mut self, cx: &mut ModelContext<Self>, mut path: Vec<PathBuf>) {
        unimplemented!()
    }

    pub fn restore_path(
        &self,
        cx: &ModelContext<Self>,
        path: PathBuf,
        inode: u64,
    ) -> Receiver<Option<UmountResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!(
            // FIXME: settings not work, currently
            "{base}/api/fs/umount",
            base = self.fuse_url
        );

        // If it panics, that means there's a bug in code.
        let path = path.to_str().unwrap();
        let req = UmountRequest {
            path: Some(path),
            inode: Some(inode),
        };
        let body = serde_json::to_string(&req).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client
                .post_json(
                    "http://127.0.0.1:2725/api/fs/umount",
                    AsyncBody::from(body),
                )
                .await
            {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<UmountResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn get_checkout_paths(&self, cx: &ModelContext<Self>) -> Receiver<Option<MountsResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!(
            // FIXME: settings not work, currently
            "{base}/api/fs/mpoint",
            base = self.fuse_url
        );

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client
                .get(
                    "http://127.0.0.1:2725/api/fs/mpoint",
                    AsyncBody::empty(),
                    false,
                )
                .await
            {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<MountsResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn get_fuse_config(&self, cx: &ModelContext<Self>) -> Receiver<Option<ConfigResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!(
            // FIXME: settings not work, currently
            "{base}/api/config",
            base = self.fuse_url
        );

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client
                .get(
                    "http://127.0.0.1:2725/api/config",
                    AsyncBody::empty(),
                    false,
                )
                .await
            {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<ConfigResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }

    pub fn set_fuse_config(&self, cx: &mut ModelContext<Self>) -> Receiver<Option<ConfigResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!(
            // FIXME: settings not work, currently
            "{base}/api/config",
            base = self.fuse_url
        );
        let config = ConfigRequest {
            mega_url: None,
            mount_path: None,
            store_path: None,
        };

        let config = serde_json::to_string(&config).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client
                .post_json("http://127.0.0.1:2725/api/config", config.into())
                .await
            {
                if resp.status().is_success() {
                    let mut buf = Vec::new();
                    resp.body_mut().read_to_end(&mut buf).await.unwrap();
                    if let Ok(config) =
                        serde_json::from_slice::<ConfigResponse>(&*buf.into_boxed_slice())
                    {
                        tx.send(Some(config)).unwrap();
                    }
                }
                return;
            }

            tx.send(None).unwrap();
        })
        .detach();

        rx
    }
    
    pub fn is_path_checkout(&self, path: &String) -> bool {
        let set = &self.checkout_path;
        set.get_ancestor(path).is_some()
    }
}
