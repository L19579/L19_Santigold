use {
    crate::{
        web, 
        HttpResponse,
    },
    serde::{
        Serialize, Deserialize,
    },
    sqlx::{
        Connection, PgPool,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct PodcastData{
    pub channel: Channel,
    pub items: Vec<Item>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Channel{
    pub id: u8,
    pub external_id: String,
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
    pub sy_update_frequency: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Item{
    pub id: String,
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

/// GET RSS feed
pub async fn feed() -> HttpResponse{
    // tester
    log::info!("/feed is reachable"); 
    HttpResponse::Ok().finish()
}

/// POST episode
pub async fn upload(
    form: web::Json<PodcastData>,
    pg_conn_pool: web::Data<PgPool>,
) -> HttpResponse {
    // tester
    log::info!("/upload is reachable. JSON received: {:?}", form.into_inner()); 
    HttpResponse::Ok().finish()
}
