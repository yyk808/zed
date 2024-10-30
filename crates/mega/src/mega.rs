// This crate delegate mega and its fuse daemon.
// The following requirements should be met:
// 
// TODO:
// 1. Only one daemon on this machine. 
//      This should be both warrantied by this module and scorpio
// 2. At least one daemon on this machine when zed startup.
// 3. Complete docs.
// 4. Add settings for this module

use std::path::{Path, PathBuf};
use std::sync::Arc;
use gpui::http_client::{AsyncBody, HttpClient};
use gpui::{AppContext, Context, EntityId, EventEmitter, ModelContext, SharedString, WindowContext};
use reqwest_client::ReqwestClient;
use serde::Serialize;
use settings::Settings;
use crate::mega_settings::MegaSettings;

mod delegate;
mod fuse;
mod mega_settings;

pub fn init(cx: &mut AppContext) {
    Mega::init(cx);
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    MegaRunning(bool),
    FuseRunning(bool),
    FuseMounted(Option<PathBuf>),
    FuseCheckout(Option<PathBuf>),
}
pub struct Mega {
    mega_running: bool,
    fuse_running: bool,
    fuse_mounted: bool,
    
    mount_point: Option<PathBuf>,
    checkout_path: Option<PathBuf>,
    
    mega_url: String,
    fuse_url: String,
}

pub struct MegaFuse {}

impl EventEmitter<Event> for Mega {}


impl Mega {
    pub fn init_settings(cx: &mut AppContext) { MegaSettings::register(cx); }
    
    pub fn init(cx: &mut AppContext) {
        Self::init_settings(cx);
    }
    
    pub fn new(cx: &mut AppContext) -> Self {
        let mount_point = PathBuf::from(MegaSettings::get_global(cx).mount_point.clone());
        let mega_url = MegaSettings::get_global(cx).mega_url.clone();
        let fuse_url = MegaSettings::get_global(cx).fuse_url.clone();
        
        Mega {
            fuse_running: false,
            mega_running: false,
            fuse_mounted: false,
            
            mount_point: None,
            checkout_path: None,
            
            mega_url,
            fuse_url,
        }
    }

    pub fn update_status(&mut self, cx: &mut ModelContext<Self>) {
        
        
        
        cx.notify();
    }

    pub fn status(&self) -> (bool, bool, bool) {
        (self.mega_running, self.fuse_running, self.fuse_mounted)
    }
    
    pub fn toggle_mega(&mut self, cx: &mut ModelContext<Self>) {
        self.mega_running = !self.mega_running;
        cx.emit(Event::MegaRunning(self.mega_running));
    }
    
    pub fn toggle_fuse(&mut self, cx: &mut ModelContext<Self>) {
        self.fuse_running = !self.fuse_running;
        cx.emit(Event::FuseRunning(self.fuse_running));
    }

    pub fn toggle_mount(&mut self, cx: &mut ModelContext<Self>) {
        // let req_body = delegate::MountRequest {
        //     path: "".parse().unwrap()
        // };

        cx.spawn(|this, mut cx| async move {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/fs/mount",
                AsyncBody::empty(),
                false
            ).await;
            
            if let Some(mega) = this.upgrade() {
                let _ = mega.update(&mut cx, |this, cx| {
                    if this.fuse_mounted {
                        this.fuse_mounted = false;
                    } else {
                        // FIXME just pretending that we've got something from fuse response
                        this.fuse_mounted = true;
                        this.mount_point = Some(PathBuf::from("/home/neon/projects"));
                    }
                    cx.emit(Event::FuseMounted(this.mount_point.clone()));
                });
            }
        }).detach();
    }
    
    pub fn checkout_path(&mut self, cx: &mut ModelContext<Self>, mut path: PathBuf) {
        // for now, we assume there's only one path being checkout at a time.
        if self.checkout_path.is_none() {
            cx.spawn(|_this, _cx| async {
                let client = ReqwestClient::new();
                let req = client.get(
                    "localhost:2725/api/fs/mount",
                    AsyncBody::empty(),
                    false
                ).await;
            }).detach();
        }
        
        
    }

    pub fn checkout_multi_path(&mut self, cx: &mut ModelContext<Self>, mut path: Vec<PathBuf>) {
        // for now, we assume there's only one path being checkout at a time.
        if self.checkout_path.is_none() {
            cx.spawn(|_this, _cx| async {
                let client = ReqwestClient::new();
                let req = client.get(
                    "localhost:2725/api/fs/mount",
                    AsyncBody::empty(),
                    false
                ).await;
            }).detach();
        }


    }

    pub fn get_fuse_config(&self, cx: &mut ModelContext<Self>) {
        cx.spawn(|_this, _cx| async {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/fs/mount",
                AsyncBody::empty(),
                false
            ).await;
        }).detach();
    }

    pub fn set_fuse_config(&self, cx: &mut ModelContext<Self>) {
        cx.spawn(|_this, _cx| async {
            let client = ReqwestClient::new();
            let req = client.post_json(
                "localhost:2725/api/config",
                AsyncBody::empty(),
            ).await;
        }).detach();
    }
    
}

#[cfg(test)]
mod test {

}
