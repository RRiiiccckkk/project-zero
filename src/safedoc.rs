use serde::{Deserialize, Serialize};

// 定义去中心化页面的结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SafePage {
    pub title: String,
    pub author: String,
    pub elements: Vec<DocElement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DocElement {
    Header(String),
    Paragraph(String),
    Link { text: String, target_key: String }, // 指向另一个 DHT Key 的链接
    CodeBlock(String),
}

impl SafePage {
    // 一个纯文本的渲染器
    pub fn render(&self) {
        println!("\n{}", "=".repeat(50));
        println!("📄  {}", self.title.to_uppercase());
        println!("✍️   By: {}", self.author);
        println!("{}", "=".repeat(50));
        println!();

        for element in &self.elements {
            match element {
                DocElement::Header(text) => {
                    println!("\x1b[1;34m# {}\x1b[0m", text); // 蓝色加粗
                    println!("{}", "-".repeat(text.len() + 2));
                },
                DocElement::Paragraph(text) => {
                    println!("  {}\n", text);
                },
                DocElement::Link { text, target_key } => {
                    println!("  🔗 \x1b[4m{} \x1b[0m -> Key: {}", text, target_key);
                },
                DocElement::CodeBlock(code) => {
                    println!("\x1b[100m\x1b[37m"); // 灰色背景
                    for line in code.lines() {
                        println!("  {}  ", line);
                    }
                    println!("\x1b[0m\n");
                }
            }
        }
        println!("{}", "=".repeat(50));
        println!("(Type 'get <key>' to download raw data, or 'browse <key>' to render)\n");
    }
    
    // 辅助：创建一个示例页面
    pub fn example() -> Self {
        Self {
            title: "Hello Web 3.0".into(),
            author: "Satoshi".into(),
            elements: vec![
                DocElement::Header("Welcome to Project Zero".into()),
                DocElement::Paragraph("This page is stored on a decentralized DHT. No server hosts this file.".into()),
                DocElement::CodeBlock("cargo run --release".into()),
                DocElement::Header("Links".into()),
                DocElement::Link { 
                    text: "My Identity".into(), 
                    target_key: "some_random_key".into() 
                }
            ]
        }
    }
}