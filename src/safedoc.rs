use serde::{Deserialize, Serialize};

// ---------------------------------------------------------
// 页面元素 (The Elements)
// ---------------------------------------------------------
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Element {
    Header(String),
    Text(String),
    Link { label: String, target_id: String },
}

// ---------------------------------------------------------
// 安全页面 (The Safe Page)
// ---------------------------------------------------------
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafePage {
    pub title: String,
    pub elements: Vec<Element>,
}

impl SafePage {
    // [修复点] 确保 new 方法存在
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            elements: Vec::new(),
        }
    }

    pub fn add(mut self, elem: Element) -> Self {
        self.elements.push(elem);
        self
    }

    // [渲染引擎]
    pub fn render(&self) {
        println!("\n╔════════════════════════════════════════╗");
        println!("║ {:^38} ║", self.title);
        println!("╠════════════════════════════════════════╣");
        
        for elem in &self.elements {
            match elem {
                Element::Header(text) => {
                    println!("║  [#] {:<34}║", text);
                    println!("║                                        ║");
                },
                Element::Text(text) => {
                    let max_len = 36;
                    if text.len() > max_len {
                        println!("║  {:<36}  ║", &text[0..max_len]);
                        println!("║  {:<36}  ║", &text[max_len..]);
                    } else {
                        println!("║  {:<36}  ║", text);
                    }
                    println!("║                                        ║");
                },
                Element::Link { label, target_id } => {
                    println!("║  >> 🔗 [{}] ---> {}  ║", label, &target_id[0..6]);
                }
            }
        }
        println!("╚════════════════════════════════════════╝\n");
    }
}