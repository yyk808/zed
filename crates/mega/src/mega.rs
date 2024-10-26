// This crate delegate mega and its fuse daemon.
// The following requirements should be met:
// 
// TODO:
// 1. Only one daemon on this machine.
// 2. At least one daemon on this machine when zed startup.
// 3. Complete docs.

use std::path::{Path, PathBuf};
use gpui::http_client::{AsyncBody, HttpClient};
use gpui::{AppContext, Context, EntityId, EventEmitter, ModelContext, WindowContext};
use reqwest_client::ReqwestClient;
use serde::Serialize;

mod delegate;
mod fuse;

pub fn init(cx: &mut AppContext) {
    // let reservation = cx.reserve_model();
    // cx.insert_model(reservation, |cx| {
    //     cx.new_model(|_cx| { Mega::new() })
    // });
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    MegaRunning(bool),
    FuseRunning(bool),
    FuseMounted(bool),
}
pub struct Mega {
    mega_running: bool,
    fuse_running: bool,
    fuse_mounted: bool,
    
    checkout_path: Option<PathBuf>,
    panel_id: Option<EntityId>,
}

pub struct MegaFuse {}

impl EventEmitter<Event> for Mega {}


impl Mega {
    pub fn init_settings(cx: &mut AppContext) {
        
    }
    
    pub fn init(cx: &mut AppContext) {
        // let reservation = cx.reserve_model();
        // cx.insert_model(reservation, |cx| {
        //     cx.new_model(|_cx| { Mega::new() })
        // });
    }
    
    pub fn new(cx: &mut AppContext) -> Self {
        Mega {
            fuse_running: false,
            mega_running: false,
            fuse_mounted: false,
            checkout_path: None,
            panel_id: None,
        }
    }
    
    pub fn update_status(&mut self, cx: &mut ModelContext<Self>) {
        if let None = self.panel_id {
            return;
        }
        
        cx.notify();
    }
    
    pub fn status(&self) -> (bool, bool, bool) {
        (self.mega_running, self.fuse_running, self.fuse_mounted)
    }
    
    pub fn toggle_mega(&self, cx: &mut ModelContext<Self>) { todo!() }
    
    pub fn toggle_fuse(&self, cx: &mut ModelContext<Self>) { 
        
    }

    pub fn toggle_mount(&self, cx: &mut ModelContext<Self>) {
        // let req_body = delegate::MountRequest {
        //     path: "".parse().unwrap()
        // };
        
        cx.spawn(|_this, _cx| async {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/fs/mount",
                AsyncBody::empty(),
                false
            ).await;
        }).detach();
    }
    
    pub fn checkout_path(&self, cx: &mut ModelContext<Self>) {
        cx.spawn(|_this, _cx| async {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/fs/mount",
                AsyncBody::empty(),
                false
            ).await;
        }).detach();
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

    pub fn get_fuse_mpoint(&self, cx: &mut ModelContext<Self>) {
        cx.spawn(|_this, _cx| async {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/config",
                AsyncBody::empty(),
                false
            ).await;
        }).detach();
    }

}

#[cfg(test)]
mod test {

}
