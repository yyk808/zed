// This crate delegate mega and its fuse daemon.
// The following requirements should be met:
// 
// TODO:
// 1. Only one daemon on this machine.
// 2. At least one daemon on this machine when zed startup.
// 3. Complete docs.

use std::sync::Arc;
use gpui::{AppContext, Context, EventEmitter, Model, ModelContext};

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
pub struct Mega {}

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
        Mega {}
    } 
    
    pub fn toggle_mega(&self) { todo!() }
    
    pub fn toggle_fuse(&self) { todo!() }
    
    pub fn checkout_path(&self) { todo!() }
}
