use {
    crate::{
        Arc, RwLock,
        web, HttpResponse,
        ContentType, Uuid
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

const NONE: String = String::from("NONE");

#[derive(Serialize, Deserialize, Debug)]
pub struct PodcastData{
    pub channel: Channel,
    pub items: Vec<Item>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Channel{
    pub id: i32,
    pub external_id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub managing_editor: String,
    pub generator: String,
    pub image_url: String,
    pub image_title : String,
    pub image_link: String,
    pub image_width : i32,
    pub image_height: i32,
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
    pub ep_number: i32,
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

/// GET RSS feed - d 
pub async fn feed(xml_buffer: web::Data<Arc<RwLock<String>>>) -> HttpResponse{
    let body: String = xml_buffer.read().unwrap().clone();
    return HttpResponse::Ok()
        .content_type(ContentType::xml())
        .body(body)
}

/// GET channels data - d 
pub async fn channels(pg_conn_pool: web::Data<PgPool>) -> HttpResponse{
    let channels: Vec<Channel> = match sqlx::query_as!( Channel, 
        r#" SELECT * FROM channel "#
    )
    .fetch_all(pg_conn_pool.get_ref())
    .await {
        Ok(chs) => chs,
        Err(_) => Vec::new()
    }; 
    
    if channels.len() < 1 {
        return HttpResponse::NoContent()
            .body("No channels in DB");
    }
    
    let mut response_ser_json = String::new();
    channels.into_iter().for_each(|c|{
        let serialized_c = serde_json::ser::to_string(&c).unwrap(); // TODO - error handling;
        response_ser_json.push_str(&serialized_c);
    });

    if channels.len() > 1 {
        response_ser_json = format!("{{{}}}", response_ser_json);
    }
    
    return HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(response_ser_json);
}

/// GET episode metadata - d
pub async fn episode(
    episode: web::Json<ItemAbbreviated>,
    pg_conn_pool: web::Data<PgPool>,
) -> HttpResponse{
    match sqlx::query_as!(
        Item,
        r#"SELECT * FROM item WHERE id = $1"#, Uuid::parse_str(&episode.into_inner().id).unwrap()
    ).fetch_optional(pg_conn_pool.get_ref())
    .await
    .unwrap(){
        Some(e) => {
            // TODO error handling
            let serialized_ep = serde_json::ser::to_string(&e).unwrap(); 
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(serialized_ep)
        },
        None => {
            return HttpResponse::NoContent()
                .body("No channels in DB");
        }
    }
}   

/// POST modify episode metadata - d
pub async fn edit(
    episode: web::Json<Item>,
    pg_conn_pool: &web::Data<PgPool>
) -> HttpResponse{
    let ep = episode.into_inner();

    if !(episode_exists(&ep.id).await.is_ok()) {
        return HttpResponse::BadRequest()
            .body("episode does not exist");
    }

    sqlx::query!(r#"
        UPDATE item SET channel_id = $1, ep_number = $2, title = $3, author = $4, description = $5,
        content_encoded = $6, enclosure = $7, i_link = $8, pub_date = $9, itunes_subtitles = $10, 
        itunes_image = $11, itunes_duration = $12 WHERE id = $13
        "#, ep.channel_id, ep.ep_number, ep.author, ep.category, ep.description, ep.content_encoded, 
        ep.enclosure, ep.i_link, ep.pub_date, ep.itunes_subtitles.unwrap_or(NONE.clone()),
        ep.itunes_image.unwrap_or(NONE.clone()), ep.itunes_duration.unwrap_or(NONE.clone()), ep.id
    ).execute().await;
    return HttpResponse::Ok().finish();
}

/// POST Channel/Episode - near // linode
pub async fn upload(
    /* episode: web::Json<Item>, */
    podcast_data: web::Json<PodcastData>,
    pg_conn_pool: web::Data<PgPool>
) -> HttpResponse {
    let podcast_data = &podcast_data.into_inner();
    let ch = &podcast_data.channel;
    let eps: &[Item] = &podcast_data.items;
    let mut potential_bad_ep_uploads = String::new();
    eps.iter().for_each(|ep|{
        if !(ch.external_id != ep.channel_id){
            potential_bad_ep_uploads
                .push_str(&format!("\n{}", ep.channel_id))
        }
    });

    if potential_bad_ep_uploads.len() != 0 {
        return HttpResponse::BadRequest()
            .body(format!("wrong channel_id's for episodes: \n{}", potential_bad_ep_uploads));
    }

    store_to_db(podcast_data, &pg_conn_pool);

    refresh_xml_buffer().unwrap(); 
    HttpResponse::Ok().finish()
}

/// check that channel exists in db and on linode. - near // linode
async fn channel_exists(ch_id: &str) -> Result<(), &'static str>{
    let res = match sqlx::query!(
        "r# SELECT id FROM channel WHERE external_id = $1 #", ch_id
    ).fetch_optional::<Channel>()
        .get_ref()
        .await{
            Some(_) => true,
            None => false,
    };
    return Ok(());
}

/// check that episode exists in db and on linode. - near // linode
async fn episode_exists(ep_id: &str) -> Result<(), &'static str>{
    let res = match sqlx::query!(
        r#" SELECT ep_number FROM item WHERE id = $1 "#, ep_id
    ).fetch_optional::<Item>()
        .get_ref()
        .await{
            Some(_) => true,
            None => false,
    };
    return Ok(());
}

/// store episode data in db - d 
async fn store_to_db(podcast_data: &PodcastData, pg_conn_pool: &web::Data<PgPool>) 
-> Result<(), &'static str>{
    //TODO temp, this should rarely fail. Error is worth attention if it does.
    //Work on error handling.
    let ch = podcast_data.channel;
    let eps: Vec<Item> = podcast_data.items;

    if !(channel_exists(&ch.external_id).await.is_ok()) {
        sqlx::query!(r#"
            INSERT INTO channel (external_id, title, category, description, managing_editor,
            generator, image_url, image_title, image_link, image_width, image_height, language,
            last_build_date, pub_date, c_link, itunes_new_feed_url, itunes_explicit, itunes_owner_name,
            itunes_owner_email, sy_update_period, sy_update_frequency)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20, $21)
            "#, Uuid::parse_str(&ch.external_id).unwrap(), ch.title, ch.category, ch.description, ch.managing_editor, ch.generator, //TODO: unwrap here.
            ch.image_url, ch.image_title, ch.image_link, ch.image_width, ch.image_height, ch.language,
            ch.last_build_date, ch.pub_date, ch.c_link, ch.itunes_new_feed_url.unwrap_or(NONE.clone()), 
            ch.itunes_explicit.unwrap_or(false), ch.itunes_owner_name.unwrap_or(NONE.clone()), 
            ch.itunes_owner_email.unwrap_or(NONE.clone()), ch.sy_update_period.unwrap_or(NONE.clone()), 
            ch.sy_update_frequency.unwrap_or(NONE.clone())
        ).execute(pg_conn_pool.get_ref())
        .await;
    } 

    sqlx::query!(r#"
        INSERT INTO item (id, channel_id, ep_number, title, author, description, content_encoded, 
        enclosure, i_link, pub_date, itunes_subtitles, itunes_image. itunes_duration)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#, ep.id, ep.channel_id, ep.ep_number, ep.author, ep.category, ep.description, 
        ep.content_encoded, ep.enclosure, ep.i_link, ep.pub_date, ep.itunes_subtitles.unwrap_or("NONE"),
        ep.itunes_image.unwrap_or("NONE"), ep.itunes_duration.unwrap_or("NONE"),
    ).execute(pg_conn_pool.get_ref())
    .await;     

    return Ok(()); 
}

/// refresh xml with updated db data
fn refresh_xml_buffer() -> Result<(), &'static str>{
    // create file if it doesn't exist;
    todo!()
}
