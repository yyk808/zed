use crate::api::{
    ConfigRequest, ConfigResponse, MountRequest, MountResponse, MountsResponse, UmountRequest,
    UmountResponse,
};
use crate::mega_settings::MegaSettings;
use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;
use futures::AsyncReadExt;
use gpui::http_client::{AsyncBody, HttpClient};
use gpui::{AppContext, EventEmitter, ModelContext};
use radix_trie::{Trie, TrieCommon};
use reqwest_client::ReqwestClient;
use settings::Settings;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

pub mod api;
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
    notify: bool,
}

pub struct Mega {
    fuse_executable: PathBuf,

    fuse_running: bool,
    fuse_mounted: bool,
    heartbeat: bool,

    mount_point: Option<PathBuf>,
    checkout_path: Trie<String, u64>,

    mega_url: String,
    fuse_url: String,
    http_client: Arc<ReqwestClient>,
}

pub struct MegaFuse {}

impl EventEmitter<Event> for Mega {}

impl Debug for Mega {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fuse_executable: {:?}, mega_url: {}, fuse_url: {}",
            self.fuse_executable, self.mega_url, self.fuse_url
        )
    }
}

impl Mega {
    pub fn init_settings(cx: &mut AppContext) {
        MegaSettings::register(cx);
    }

    pub fn init(cx: &mut AppContext) {
        Self::init_settings(cx);
    }

    pub fn new(cx: &mut AppContext) -> Self {
        let mount_path = MegaSettings::get_global(cx).mount_point.clone();
        let mega_url = MegaSettings::get_global(cx).mega_url.clone();
        let fuse_url = MegaSettings::get_global(cx).fuse_url.clone();
        let fuse_executable = MegaSettings::get_global(cx).fuse_executable.clone();

        // To not affected by global proxy settings.
        let client = ReqwestClient::new();

        let mount_point = if mount_path.exists() {
            Some(mount_path)
        } else {
            None
        };

        let mega = Mega {
            fuse_executable,

            fuse_running: false,
            fuse_mounted: false,
            heartbeat: false,

            mount_point,
            checkout_path: Default::default(),

            mega_url,
            fuse_url,
            http_client: Arc::new(client),
        };

        println!("Mega New: {mega:?}");
        mega
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
                            mega.fuse_mounted = false;
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
                                    cx.emit(Event::FuseCheckout(Some(PathBuf::from(
                                        i.path.clone(),
                                    ))))
                                }
                            }
                        })
                    }
                }
            } else {
                Ok(())
            }
            .unwrap();

            // When mount point changed, emit an event.
            // update mount point if it's none.
            if let Ok(Some(config)) = config.await {
                this.update(&mut cx, |this, cx| {
                    let path = PathBuf::from(config.config.mount_path);
                    if (this.fuse_mounted && this.fuse_running) && this.mount_point.is_some() {
                        if let Some(inner) = &this.mount_point {
                            if !inner.eq(&path) {
                                this.mount_point = Some(path);
                                cx.emit(Event::FuseMounted(this.mount_point.clone()));
                            }
                        }
                    } else if this.fuse_running && this.mount_point.is_none() {
                        this.mount_point = Some(path);
                        cx.emit(Event::FuseMounted(this.mount_point.clone()));
                    }
                })
            } else {
                Ok(())
            }
        })
        .detach();
    }

    pub fn status(&self) -> (bool, bool) {
        (self.fuse_running, self.fuse_mounted)
    }

    /// ## Toggle Fuse checkouts
    /// Checkout or un-checkout the paths in zed.
    /// Does nothing if fuse not running.
    pub fn toggle_fuse(&mut self, cx: &mut ModelContext<Self>) {
        self.update_status(cx);
        let paths = &self.checkout_path;

        if !self.fuse_running {
            return;
        }

        if !self.fuse_mounted {
            for (_, (p, _)) in paths.iter().enumerate() {
                let path = PathBuf::from(p); // FIXME is there a better way?
                cx.spawn(|mega, mut cx| async move {
                    let recv = mega
                        .update(&mut cx, |this, cx| this.checkout_path(cx, path))
                        .expect("mega delegate not be dropped");

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
            cx.emit(Event::FuseMounted(self.mount_point.clone()));
        } else {
            for (_, (p, _)) in paths.iter().enumerate() {
                let path = PathBuf::from(p); // FIXME is there a better way?
                cx.spawn(|mega, mut cx| async move {
                    let recv = mega
                        .update(&mut cx, |this, cx| this.restore_path(cx, path))
                        .expect("mega delegate not be dropped");

                    if let Ok(Some(_resp)) = recv.await {
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
        }
    }

    /// ## Toggle Fuse Mount
    /// In fact, we cannot `mount` or `umount` a fuse from zed.
    ///
    /// This function only opens up a new scorpio executable if it detects fuse not running.
    pub fn toggle_mount(&mut self, cx: &mut ModelContext<Self>) {
        // We only start it, not stop it.
        if !self.fuse_running {
            let _ = Command::new(self.fuse_executable.as_os_str())
                .spawn()
                .expect("Fuse Executable path not right");

            self.update_status(cx);
        }
    }

    pub fn checkout_path(
        &self,
        cx: &ModelContext<Self>,
        path: PathBuf,
    ) -> Receiver<Option<MountResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!("{base}/api/fs/mount", base = self.fuse_url);

        // If it panics, that means there's a bug in code.
        let path = path.to_str().unwrap();
        let req = MountRequest { path };
        let body = serde_json::to_string(&req).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.post_json(uri.as_str(), AsyncBody::from(body)).await {
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

    pub fn restore_path(
        &self,
        cx: &ModelContext<Self>,
        path: PathBuf,
    ) -> Receiver<Option<UmountResponse>> {
        let (tx, rx) = oneshot::channel();
        let client = self.http_client.clone();
        let uri = format!("{base}/api/fs/umount", base = self.fuse_url);

        // If panics here, that means there's a bug in code.
        // maybe we should ensure every path absolute?
        let path = path.to_str().unwrap();
        let inode = self.checkout_path.get_ancestor_value(path);
        let req = UmountRequest {
            path: Some(path),
            inode: Some(inode.unwrap().to_owned()),
        };
        let body = serde_json::to_string(&req).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.post_json(uri.as_str(), AsyncBody::from(body)).await {
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
        let uri = format!("{base}/api/fs/mpoint", base = self.fuse_url);

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.get(uri.as_str(), AsyncBody::empty(), false).await {
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
        let uri = format!("{base}/api/config", base = self.fuse_url);

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.get(uri.as_str(), AsyncBody::empty(), false).await {
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
        let uri = format!("{base}/api/config", base = self.fuse_url);
        let config = ConfigRequest {
            mega_url: None,
            mount_path: None,
            store_path: None,
        };

        let config = serde_json::to_string(&config).unwrap();

        cx.spawn(|_this, _cx| async move {
            if let Ok(mut resp) = client.post_json(uri.as_str(), config.into()).await {
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

    pub fn heartbeat(&mut self, cx: &mut ModelContext<Self>) {
        if self.heartbeat {
            return;
        } else {
            self.heartbeat = true;
        }

        cx.spawn(|this, mut cx| async move {
            loop {
                this.update(&mut cx, |mega, cx| {
                    mega.update_status(cx);
                })
                .expect("mega delegate not be dropped");

                let dur = Duration::from_secs(30);
                cx.background_executor().timer(dur).await;
            }
        })
        .detach();
    }

    pub fn is_path_checkout(&self, path: PathBuf) -> bool {
        let set = &self.checkout_path;

        set.get_ancestor(path.to_str().unwrap()).is_some()
    }

    pub fn mark_checkout(&mut self, cx: &mut ModelContext<Self>, path: String, inode: u64) {
        if self.mount_point.is_some() {
            let path = self
                .mount_point
                .clone()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
                + path.as_str();
            self.checkout_path.insert(path, inode);
            cx.emit(Event::FuseCheckout(None));
        }
    }

    pub fn mark_commited(&mut self, cx: &mut ModelContext<Self>, path: PathBuf) {
        self.checkout_path.remove(path.to_str().unwrap());
        cx.emit(Event::FuseCheckout(None));
    }
}
