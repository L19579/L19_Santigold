use {
    crate::{
        Arc, RwLock,
        web, HttpResponse,
        ContentType, S3Client,
    },
    serde::{
        Serialize, Deserialize,
    },
    sqlx::{
        Connection, PgPool,
        types::Uuid,
    },
    uuid::Uuid as native_Uuid,
    std::{
        time::Duration,
        fs::File,
        result::Result,
    },
    
};

pub fn none() -> String{
    "NONE".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PodcastData{
    pub channel: Channel,
    pub items: Vec<Item>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Channel{
    pub id: i32,
    //#[serde(deserialize_with = "deserialize_uuid")]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Item{
    pub id: String,
    pub channel_id: String,
    pub ep_number: i32,
    pub title: String,
    pub author: String,
    pub category: String,
    pub description: String,
    pub content_encoded: String,
    pub enclosure: String,
    pub i_link: String,
    pub pub_date: String,
    pub itunes_subtitle: Option<String>,
    pub itunes_image: Option<String>,
    pub itunes_duration: Option<String>,
}

#[derive(Serialize, Deserialize, Clone,Debug)]
pub struct ItemAbbreviated{
    pub id: String,
    pub channel_id: u8,
}

pub struct S3{
    pub client: S3Client,
    pub bucket: String,
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
    let channels: Vec<_> =  match sqlx::query!(
        r#" SELECT * FROM channel "#
    )
    .fetch_all(pg_conn_pool.get_ref())
    .await{
        Ok(chs) => chs,
        Err(_) => Vec::new()
    };

    if channels.len() < 1 {
        return HttpResponse::NoContent()
            .body("No channels in DB");
    }

    let mut response_ser_json = String::new();
    channels.into_iter().for_each(|c|{ // TODO: NOT ideal.
        let ch = Channel {
            id: c.id,
            external_id: c.external_id.to_string(),
            title: c.title,
            category: c.category,
            description: c.description,
            managing_editor: c.managing_editor,
            generator: c.generator,
            image_url: c.image_url,
            image_title : c.image_title,
            image_link: c.image_link,
            image_width : c.image_width,
            image_height: c.image_height,
            language: c.language,
            last_build_date: c.last_build_date,
            pub_date: c.pub_date,
            c_link: c.c_link,
            itunes_new_feed_url: Some(c.itunes_new_feed_url),
            itunes_explicit: Some(c.itunes_explicit),
            itunes_owner_name: Some(c.itunes_owner_name),
            itunes_owner_email: Some(c.itunes_owner_email),
            sy_update_period: Some(c.sy_update_period),
            sy_update_frequency: Some(c.sy_update_frequency),
        
        };
            
        let serialized_c = serde_json::ser::to_string(&ch).unwrap(); // TODO - error handling;
        response_ser_json.push_str(&serialized_c);
        /* 
        let serialized_c = serde_json::ser::to_string(&c).unwrap(); // TODO - error handling;
        response_ser_json.push_str(&serialized_c);
        */
    });

    if response_ser_json.len() > 1 {
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
    let res = match sqlx::query!(
        r#"SELECT * FROM item WHERE id = $1"#,
        Uuid::parse_str(&episode.into_inner().id).unwrap() // TODO: error caused by UUID ser issue
    ).fetch_optional(pg_conn_pool.get_ref())
    .await
    .unwrap(){
        Some(e) => {
            e
        },
        None => {
            return HttpResponse::NoContent()
                .body("No channels in DB");
        }
    };

    // No clean way to do this right now. Refactor.
    let ep = Item{
        id: res.id.to_string(),
        channel_id: res.channel_id.to_string(),
        description: res.description,
        ep_number: res.ep_number,
        title: res.title,
        author: res.author,
        category: res.category,
        content_encoded: res.content_encoded,
        enclosure: res.enclosure,
        i_link: res.i_link,
        pub_date: res.pub_date,
        itunes_subtitle: Some(res.itunes_subtitle),
        itunes_image: Some(res.itunes_image),
        itunes_duration: Some(res.itunes_duration), // TODO: correct, we won't always expect this to be filled.
    };

    let response_ser_json = serde_json::ser::to_string(&ep).unwrap(); 
    let response_ser_json = format!("{{{}}}", response_ser_json);

    return HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(response_ser_json);

    /* HttpResponse::Ok() // --- cleaner tbh
        .json(res_body);
    */
}

/// POST modify episode metadata - d
pub async fn edit_episode(
    updated_ep: web::Json<Item>,
    pg_conn_pool: &web::Data<PgPool>,
    s3: &web::Data<S3>
) -> HttpResponse{
    let ep = updated_ep.into_inner();

    if !(episode_exists(&Uuid::parse_str(&ep.id).unwrap(), &pg_conn_pool, &s3).await) { // TODO refactor
        return HttpResponse::BadRequest()
            .body("episode does not exist");
    }

    match sqlx::query!(r#"
        UPDATE item SET channel_id = $1, ep_number = $2, title = $3, author = $4, description = $5,
        content_encoded = $6, enclosure = $7, i_link = $8, pub_date = $9, itunes_subtitle = $10,
        itunes_image = $11, itunes_duration = $12 WHERE id = $13
        "#, Uuid::parse_str(&ep.channel_id).unwrap(), ep.ep_number, ep.author, ep.category, ep.description,
        ep.content_encoded, ep.enclosure, ep.i_link, ep.pub_date, ep.itunes_subtitle.unwrap_or(none()),
        ep.itunes_image.unwrap_or(none()), ep.itunes_duration.unwrap_or(none()),
        Uuid::parse_str(&ep.id).unwrap()
    ).execute(pg_conn_pool.get_ref()).await{
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to edit DB"),
    }
}

/// POST modify channel metadata - d
pub async fn edit_channel(
    updated_ch: web::Json<Channel>,
    pg_conn_pool: &web::Data<PgPool>,
) -> HttpResponse {
    let ch = updated_ch.into_inner();
    if!(channel_exists(&Uuid::parse_str(&ch.external_id).unwrap(), &pg_conn_pool).await) {
        return HttpResponse::BadRequest()
            .body("channel does not exist");
    };

    match sqlx::query!(r#"
            UPDATE channel SET title = $1, category = $2, description = $3,  
            managing_editor = $4, generator = $5, image_url = $6, image_title = $7, 
            image_link = $8, image_width = $9, image_height = $10, language = $11, 
            last_build_date = $12, pub_date = $13, c_link = $14, itunes_new_feed_url = $15, 
            itunes_explicit = $16, itunes_owner_name = $17, itunes_owner_email = $18, 
            sy_update_period = $19, sy_update_frequency = $20 WHERE external_id = $21
        "#, ch.title, ch.category, ch.description, ch.managing_editor, ch.generator, 
        ch.image_url, ch.image_title, ch.image_link, ch.image_width, ch.image_height,
        ch.language, ch.last_build_date, ch.pub_date, ch.c_link, ch.itunes_new_feed_url.unwrap_or(none()),
        ch.itunes_explicit.unwrap_or(false), ch.itunes_owner_name.unwrap_or(none()), 
        ch.itunes_owner_email.unwrap_or(none()), ch.sy_update_period.unwrap_or(none()),
        ch.sy_update_frequency.unwrap_or(none()), Uuid::parse_str(&ch.external_id).unwrap()
    ).execute(pg_conn_pool.get_ref()).await{
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to edit DB"),
    }
}

/// POST Channel/Episode - near // linode
pub async fn upload(
    /* episode: web::Json<Item>, */
    podcast_data: web::Json<PodcastData>,
    pg_conn_pool: web::Data<PgPool>,
    xml_buffer: web::Data<Arc<RwLock<String>>>
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

    store_to_db(podcast_data, &pg_conn_pool).await.unwrap();

    refresh_xml_buffer(&xml_buffer).unwrap();
    HttpResponse::Ok().finish()
}

/// check that channel exists in db and on linode. - near
async fn channel_exists(ch_id: &Uuid, pg_conn_pool: &web::Data<PgPool>
)-> bool{
    match sqlx::query!(
        r#" SELECT id FROM channel WHERE external_id = $1 "#, ch_id
    ).fetch_optional(pg_conn_pool.get_ref())
        .await
        .unwrap(){ // TODO: unwrap
            Some(_) => true,
            None => false,
    }
}

/// check that episode exists in db and on linode. - d / messy
async fn episode_exists(
    ep_id: &Uuid, 
    pg_conn_pool: &web::Data<PgPool>,
    s3: &web::Data<S3>
) -> bool{
    _ = match sqlx::query!(
        r#" SELECT ep_number FROM item WHERE id = $1 "#, ep_id
    ).fetch_optional(pg_conn_pool.get_ref())
        .await
        .unwrap(){
            Some(_) => (),
            None => { return false; },
    };
    
    let (s3_client, s3_bucket) = (
        s3.get_ref().client.clone(),
        s3.get_ref().bucket.clone(),
    );

    match s3_client
        .get_object_acl()
        .key(format!("{}.mp3", ep_id.to_string()))
        .bucket(s3_bucket)
        .send()
        .await{
            Ok(_) => true,
            Err (_) => false,
    }
}

/// store episode data in db - d
async fn store_to_db(
    podcast_data: &PodcastData, 
    pg_conn_pool: &web::Data<PgPool>,
)-> Result<(), &'static str>{
    //TODO temp, this should rarely fail. Error is worth attention when it does.
    //Work on error handling.
    let ch = podcast_data.channel.clone(); // redo.
    let eps: &[Item] = &podcast_data.items;

    if !(channel_exists(&Uuid::parse_str(&ch.external_id).unwrap(), &pg_conn_pool).await){
        sqlx::query!(r#"
            INSERT INTO channel (external_id, title, category, description, managing_editor,
            generator, image_url, image_title, image_link, image_width, image_height, language,
            last_build_date, pub_date, c_link, itunes_new_feed_url, itunes_explicit, itunes_owner_name,
            itunes_owner_email, sy_update_period, sy_update_frequency)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20, $21)
            "#, Uuid::parse_str(&ch.external_id).unwrap(), ch.title, ch.category, ch.description, 
            ch.managing_editor, ch.generator, ch.image_url, ch.image_title, ch.image_link, ch.image_width, 
            ch.image_height, ch.language,ch.last_build_date, ch.pub_date, ch.c_link, 
            ch.itunes_new_feed_url.unwrap_or(none()), ch.itunes_explicit.unwrap_or(false), 
            ch.itunes_owner_name.unwrap_or(none()), ch.itunes_owner_email.unwrap_or(none()), 
            ch.sy_update_period.unwrap_or(none()), ch.sy_update_frequency.unwrap_or(none())
        ).execute(pg_conn_pool.get_ref())
        .await;
    }
   
    for ep in eps{ // messy
        sqlx::query!(r#"
            INSERT INTO item (id, channel_id, ep_number, title, author, category, description, content_encoded,
            enclosure, i_link, pub_date, itunes_subtitle, itunes_image, itunes_duration)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#, Uuid::parse_str(&ep.id).unwrap(), Uuid::parse_str(&ep.channel_id).unwrap(), ep.ep_number, ep.title, 
            ep.author, ep.category, ep.description, ep.content_encoded, ep.enclosure, ep.i_link, ep.pub_date, 
            ep.itunes_subtitle.clone().unwrap_or(none()), ep.itunes_image.clone().unwrap_or(none()), 
            ep.itunes_duration.clone().unwrap_or(none()),
        ).execute(pg_conn_pool.get_ref())
        .await;
    };

    return Ok(());
}

/// refresh xml with updated db data
fn refresh_xml_buffer(xml_buffer: &web::Data<Arc<RwLock<String>>>) -> Result<(), &'static str>{
    
    todo!()
}
