// This crate delegate mega and its fuse daemon.
// The following requirements should be met:
//
// TODO:
// 1. Only one daemon on this machine.
//      This should be both warrantied by this module and scorpio
// 2. At least one daemon on this machine when zed startup.
// 3. Complete docs.
// 4. Add settings for this module

use crate::api::{ConfigRequest, ConfigResponse, MountResponse, MountsResponse};
use crate::mega_settings::MegaSettings;
use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;
use futures::{AsyncReadExt, FutureExt, SinkExt, TryFutureExt};
use gpui::http_client::{
    AsyncBody, HttpClient, HttpRequestExt
    ,
};
use gpui::{
    AppContext, Context, EventEmitter, ModelContext,
};
use reqwest_client::ReqwestClient;
use schemars::_private::NoSerialize;
use serde::Serialize;
use settings::Settings;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

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
pub struct Mega {
    fuse_running: bool,
    fuse_mounted: bool,

    mount_point: Option<PathBuf>,
    checkout_path: Vec<PathBuf>,

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
            checkout_path: Vec::new(),

            mega_url,
            fuse_url,
            http_client: Arc::new(client),
        }
    }

    pub fn update_status(&mut self, cx: &mut ModelContext<Self>) {
        let recv = self.get_mount_point(cx);
        
        fn merge_trie(mut a: &Vec<PathBuf>, b: &Vec<PathBuf> ) {
            todo!()
        }

        cx.spawn(|this, mut cx| async move {
            if let Ok(opt) = recv.await {
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
                        this.update(&mut cx, |mega, cx| {
                            // merge them
                        })
                    }
                }
            } else {
                Ok(())
            }
        })
        .detach();
    }

    pub fn status(&self) -> (bool, bool) {
        (self.fuse_running, self.fuse_mounted)
    }

    pub fn toggle_fuse(&mut self, cx: &mut ModelContext<Self>) {
        // FIXME should be able to restart fuse
        self.fuse_running = !self.fuse_running;
        cx.emit(Event::FuseRunning(self.fuse_running));
    }

    pub fn toggle_mount(&mut self, cx: &mut ModelContext<Self>) {
        // TODO traverse checkout paths and toggle them
        // persistently store them?
        
        cx.spawn(|this, mut cx| async move {
            // let client = ReqwestClient::new();
            // let req = client.get(
            //     "localhost:2725/api/fs/mount",
            //     AsyncBody::empty(),
            //     false
            // ).await;
            //
            // if let Some(mega) = this.upgrade() {
            //     let _ = mega.update(&mut cx, |this, cx| {
            //         if this.fuse_mounted {
            //             this.fuse_mounted = false;
            //         } else {
            //             // FIXME just pretending that we've got something from fuse response
            //             this.fuse_mounted = true;
            //             this.mount_point = Some(PathBuf::from("/home/neon/projects"));
            //         }
            //         cx.emit(Event::FuseMounted(this.mount_point.clone()));
            //     });
            // }
        })
        .detach();
    }

    pub fn checkout_path(&mut self, cx: &mut ModelContext<Self>, mut path: PathBuf) -> Receiver<Option<MountResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!(
            // FIXME: settings not work, currently
            "{base}/api/fs/mount",
            base = self.fuse_url
        );

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client
                .get(
                    "http://127.0.0.1:2725/api/fs/mount",
                    AsyncBody::empty(),
                    false,
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

    pub fn get_mount_point(&mut self, cx: &mut ModelContext<Self>) -> Receiver<Option<MountsResponse>> {
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

    pub fn get_fuse_config(
        &mut self,
        cx: &mut ModelContext<Self>,
    ) -> Receiver<Option<ConfigResponse>> {
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
                .post_json(
                    "http://127.0.0.1:2725/api/config",
                    config.into()
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
}
