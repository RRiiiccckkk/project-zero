// src/zeroui.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blueprint {
    pub title: String,
    pub root: Widget,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Widget {
    Container {
        direction: Direction,
        children: Vec<Widget>,
        padding: Option<f32>,
        spacing: Option<f32>,
    },
    Text {
        content: String,
        size: Option<f32>,
        color: Option<String>,
    },
    Input {
        id: String,
        placeholder: String,
    },
    Button {
        label: String,
        action_op: String,
        action_param: String,
    },
}

pub fn splash_screen(local_port: u16) -> Blueprint {
    Blueprint {
        title: format!("Project Zero (Port: {})", local_port),
        root: Widget::Container {
            direction: Direction::Vertical,
            padding: Some(30.0),
            spacing: Some(15.0),
            children: vec![
                Widget::Text { 
                    content: "ZERO UI :: SYSTEM READY".into(), 
                    size: Some(28.0), 
                    color: Some("#55AAFF".into()) 
                },
                Widget::Text { 
                    content: "Enter Target IP to fetch UI:".into(), 
                    size: None, color: None 
                },
                Widget::Input { 
                    id: "target_ip".into(), 
                    placeholder: "127.0.0.1:8080".into() 
                },
                Widget::Button { 
                    label: "CONNECT VIA UDP".into(), 
                    action_op: "connect_ip".into(),
                    action_param: "target_ip".into() 
                }
            ],
        },
    }
}