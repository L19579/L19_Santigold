use {
    uuid::Uuid,
    serde::{
        Serialize, Deserialize,
    },
}

#[derive(Serialize, Deserialize)]
pub struct PodcastData{
    pub channel: Channel,
    pub items: Vec<Item>
}

#[derive(Serialize, Deserialize)]
pub struct Channel{
    pub id: u8,
    pub external_id: Uuid,
    pub title: String,
    pub category: String,
    pub description: String,
    pub managingEditor: String,
    pub generator: String,
    pub image_url: String,
    pub image_title : String,
    pub image_link: String,
    pub image_width : u8,
    pub image_height: u8,
    pub language: String,
    pub last_build_date: String,
    pub pub_date: String,
    pub c_link: String,
    pub itunes_new_feed_url: Option<String>,
    pub itunes_explicit: Option<bool>,
    pub itunes_owner_name: Option<String>,
    pub itunes_owner_email: Option<String>,
    pub sy_update_period: Option<String>,
    pub sy__update_frequency: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Item{
    pub id: Uuid,
    pub channel_id: u8,
    pub author: String,
    pub category: String,
    pub description: String,
    pub content_encoded: String,
    pub enclosure: String,
    pub i_link: String,
    pub pub_date: String,
    pub itunes_subtitles: Option<String>,
    pub itunes_image: Option<String>,
    pub itunes_duration: Option<String>,
}
