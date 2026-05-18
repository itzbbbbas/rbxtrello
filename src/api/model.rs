use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub desc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub closed: bool,
    #[serde(default)]
    pub pos: f64,
    #[serde(default, rename = "idBoard")]
    pub id_board: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub closed: bool,
    #[serde(default, rename = "idList")]
    pub id_list: String,
    #[serde(default, rename = "idLabels")]
    pub id_labels: Vec<String>,
    #[serde(default)]
    pub pos: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default, rename = "idBoard")]
    pub id_board: String,
}
