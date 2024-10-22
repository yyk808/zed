// This crate delegate mega and its fuse daemon.
// The following requirements should be met:
// 
// TODO:
// 1. Only one daemon on this machine.
// 2. At least one daemon on this machine when zed startup.
// 3. Complete docs.

use gpui::http_client::{AsyncBody, HttpClient};
use gpui::{AppContext, Context, EventEmitter, WindowContext};
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
pub enum Event {}
pub struct Mega {
    mega_running: bool,
    fuse_running: bool,
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
        }
    } 
    
    pub fn toggle_mega(&self, cx: &mut WindowContext) { todo!() }
    
    pub fn toggle_fuse(&self, cx: &mut WindowContext) { todo!() }

    pub fn toggle_mount(&self, cx: &mut WindowContext) {
        // let req_body = delegate::MountRequest {
        //     path: "".parse().unwrap()
        // };
        
        cx.spawn(|_cx| async {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/fs/mount",
                AsyncBody::empty(),
                false
            ).await;
        }).detach();
    }
    
    pub fn checkout_path(&self, cx: &mut WindowContext) {
        cx.spawn(|_cx| async {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/fs/mount",
                AsyncBody::empty(),
                false
            ).await;
        }).detach();
    }

    pub fn get_fuse_config(&self, cx: &mut WindowContext) {
        cx.spawn(|_cx| async {
            let client = ReqwestClient::new();
            let req = client.get(
                "localhost:2725/api/fs/mount",
                AsyncBody::empty(),
                false
            ).await;
        }).detach();
    }

    pub fn set_fuse_config(&self, cx: &mut WindowContext) {
        cx.spawn(|_cx| async {
            let client = ReqwestClient::new();
            let req = client.post_json(
                "localhost:2725/api/config",
                AsyncBody::empty(),
            ).await;
        }).detach();
    }

    pub fn get_fuse_mpoint(&self, cx: &mut WindowContext) {
        cx.spawn(|_cx| async {
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
