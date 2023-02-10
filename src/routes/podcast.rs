use {
    crate::{
        Arc, RwLock,
        web, HttpResponse,
        ContentType,
    },
    serde::{
        Serialize, Deserialize,
    },
    sqlx::{
        Connection, PgPool,
    },
    std::{
        fs::File,
        result::Result,
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
    pub channel_id: String,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemAbbreviated{
    pub id: String,
    pub channel_id: u8,
}

/// GET RSS feed
pub async fn feed(xml_buffer: web::Data<Arc<RwLock<String>>>) -> HttpResponse{
    let body: String = xml_buffer.read().unwrap().clone();
    return HttpResponse::Ok()
        .content_type(ContentType::xml())
        .body(body)
}

/// GET channels data
pub async fn channels(pg_conn_pool: web::Data<PgPool>) -> HttpResponse{
    let channels: Vec<Channel> = sqlx::query!(r#"
        SELECT * FROM channel
        "#
    ).fetch_all(pg_conn_pool.get_ref())
    .await; 
    let response_ser_json = String::from("{");
    let http_response = HttpResponse::Ok;
    channels.into_iter().for_each(|c|{
        let serialized_c = match serde_json::ser::to_string(&c){
            Ok(c) => &c,
            Err(_) => {
                return HttpResponse::InternalServerError() // TODO return no bueno.
                    .body("Error building channels Json");
            }
        };
        response_ser_json.push_str(serialized_c);
    });
    response_ser_json.push_str("}");
    
    return HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(response_ser_json);
}

/// GET episode metadata
pub async fn episode(
    episode: web::Json<ItemAbbreviated>,
    pg_conn_pool: web::Data<PgPool>,
) -> HttpResponse{
    todo!();
}

/// POST modify episode metadata
pub async fn edit(
    episode: web::Json<Item>,
    pg_conn_pool: web::Data<PgPool>
) -> HttpResponse{
    todo!();
}

/// POST Channel/Episode
pub async fn upload(
    episode: web::Json<Item>,
    pg_conn_pool: web::Data<PgPool>
) -> HttpResponse {
    
    let ep = episode.into_inner();
    if !channel_exists(ep.channel_id){
        return HttpResponse::InternalServerError()
            .body("Channel does not exist")
    }

    sqlx::query!(r#"
        INSERT INTO item (id, channel_id, title, author, description, content_encoded, 
        enclosure, i_link, pub_date, itunes_subtitles, itunes_image. itunes_duration)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#, ep.id, ep.channel_id, ep.author, ep.category, ep.description, ep.content_encoded,
        ep.enclosure, ep.i_link, ep.pub_date, ep.itunes_subtitles.unwrap_or("NONE"),
        ep.itunes_image.unwrap_or("NONE"), ep.itunes_duration.unwrap_or("NONE"),
    ).execute(pg_conn_pool.get_ref())
    .await;     
    //TODO temp, this should rarely fail. Error is worth attention if it does.
    //Work on error handling.
    refresh_xml_buffer().unwrap(); 
    HttpResponse::Ok().finish()
}

/// check that channel exists in db and on linode.
fn channel_exists(channel_id: String) -> bool{
    todo!()
}

/// store episode data in db
fn store_to_db(podcast_data: &PodcastData) -> Result<(), &'static str>{
    todo!()
}

/// refresh xml with updated db data
fn refresh_xml_buffer() -> Result<(), &'static str>{
    // create file if it doesn't exist;
    todo!()
}
