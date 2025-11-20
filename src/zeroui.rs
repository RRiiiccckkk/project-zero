use serde::{Deserialize, Serialize};
use eframe::egui;
use std::collections::HashMap;

/// 定义原子交互动作
#[derive(Debug, Clone)]
pub enum Action {
    Navigate(String),
    LoadResource(String),
    // 新增：状态突变
    MutateState(StateMutation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StateMutation {
    /// 整数自增: key = key + 1
    Increment { key: String },
    /// 布尔切换: key = !key
    Toggle { key: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Component {
    VStack { children: Vec<Component>, #[serde(default)] spacing: f32 },
    HStack { children: Vec<Component>, #[serde(default)] spacing: f32 },
    
    Text { 
        /// 如果以 "$" 开头，则从 State 中读取变量
        value: String, 
        #[serde(default)] size: f32 
    },
    
    Button { 
        label: String, 
        /// 点击触发的动作
        #[serde(default)] 
        on_click: Option<ClickAction>,
    },
    
    Image { src: String, #[serde(default)] width: Option<f32>, #[serde(default)] height: Option<f32> },
    Unknown,
}

/// 按钮点击的具体行为定义
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClickAction {
    Navigate { cid: String },
    Increment { key: String },
    Toggle { key: String },
}

/// 渲染上下文：包含资源和当前状态
pub struct RenderContext<'a> {
    pub textures: &'a HashMap<String, egui::TextureHandle>,
    pub state: &'a HashMap<String, i32>, // 简化：我们只支持 i32 状态 (0=false, 1=true, N=number)
}

pub fn render(ui: &mut egui::Ui, ctx: &RenderContext, component: &Component) -> Option<Action> {
    match component {
        Component::VStack { children, spacing } => {
            let mut action = None;
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = *spacing;
                for child in children {
                    if let Some(a) = render(ui, ctx, child) { action = Some(a); }
                }
            });
            action
        }
        Component::HStack { children, spacing } => {
            let mut action = None;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = *spacing;
                for child in children {
                    if let Some(a) = render(ui, ctx, child) { action = Some(a); }
                }
            });
            action
        }
        Component::Text { value, size } => {
            // 核心逻辑：变量绑定
            let display_text = if value.starts_with('$') {
                let key = &value[1..]; // 去掉 $
                // 查找状态，找不到默认显示 0
                ctx.state.get(key).map(|v| v.to_string()).unwrap_or_else(|| "0".to_string())
            } else {
                value.clone()
            };

            let mut text = egui::RichText::new(display_text);
            if *size > 0.0 { text = text.size(*size); }
            ui.label(text);
            None
        }
        Component::Button { label, on_click } => {
            if ui.button(label).clicked() {
                if let Some(act) = on_click {
                    return match act {
                        ClickAction::Navigate { cid } => Some(Action::Navigate(cid.clone())),
                        ClickAction::Increment { key } => Some(Action::MutateState(StateMutation::Increment { key: key.clone() })),
                        ClickAction::Toggle { key } => Some(Action::MutateState(StateMutation::Toggle { key: key.clone() })),
                    };
                }
            }
            None
        }
        Component::Image { src, width, height } => {
            if let Some(texture) = ctx.textures.get(src) {
                let mut img = egui::Image::new(texture);
                if let Some(w) = width { 
                    img = img.fit_to_exact_size(egui::vec2(*w, height.unwrap_or(*w))); 
                }
                ui.add(img);
                None
            } else {
                ui.horizontal(|ui| { ui.spinner(); ui.label("Img..."); });
                Some(Action::LoadResource(src.clone()))
            }
        }
        Component::Unknown => {
            ui.colored_label(egui::Color32::RED, "Unknown");
            None
        }
    }
}

pub fn parse_blueprint(json: &str) -> Option<Component> {
    serde_json::from_str(json).ok()
}