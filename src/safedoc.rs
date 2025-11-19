use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Element {
    Header(String),
    Text(String),
    Link { label: String, target: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SafePage {
    pub title: String,
    pub elements: Vec<Element>,
}

impl SafePage {
    pub fn render(&self) {
        println!("\n╔════════════════════════════════════════╗");
        println!("║ {:^38} ║", self.title);
        println!("╠════════════════════════════════════════╣");
        
        for element in &self.elements {
            match element {
                Element::Header(h) => println!("║ [#] {:<34} ║", h),
                Element::Text(t)   => println!("║     {:<34} ║", t),
                Element::Link { label, target } => {
                    println!("║ [>] {} -> {:<23} ║", label, target)
                },
            }
        }
        println!("╚════════════════════════════════════════╝\n");
    }
}