use {
    crate::{
        Arc, RwLock,
        web, HttpResponse,
        ContentType, S3Client,
        Multipart, ByteStream,
        ObjectCannedAcl, ActiveTokens,
        is_valid_token, 
        MultipartForm,
        /* MultipartCollect, */
        MultipartFormJson,
        MultipartFormText,
        MultipartFormTempFile,
    },
    serde::{
        Serialize, Deserialize,
    },
    futures::{
        StreamExt, TryStreamExt,
    },
    sqlx::{
        PgPool, types::Uuid,
    },
    std::{
        fs,
        result::Result,
        io::Write,
        path::Path,
        fmt::Display,
        sync::Mutex,
    },

   
};

#[derive(MultipartForm)]
pub struct PodcastDataV2{
    pub podcast_data: MultipartFormJson<PodcastData>,
    pub audio: MultipartFormTempFile,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PodcastData{
    pub channel: Channel,
    pub item: Item,
    pub session_token: String,
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
    // optional
    pub itunes_new_feed_url: String,
    pub itunes_explicit: bool,
    pub itunes_owner_name: String,
    pub itunes_owner_email: String,
    pub sy_update_period: String,
    pub sy_update_frequency: String,
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
    pub enclosure_url: String,
    pub enclosure_type: String,
    pub enclosure_length: String,
    pub i_link: String,
    pub pub_date: String,
    //optional; maybe not. Podcatchers weirdly reliant on itune tags
    pub itunes_subtitle: String,
    pub itunes_image: String,
    pub itunes_duration: String,
}

#[derive(Serialize, Deserialize, Clone,Debug)]
pub struct ItemAbbreviated{
    pub id: String,
    pub channel_id: u8,
}

#[derive(Clone, Debug)]
pub struct S3{
    pub client: S3Client,
    pub bucket: String,
    pub full_link: String,
    pub temp_dir: String,
}

#[derive(Serialize, Deserialize, Clone,Debug)]
struct UploadObjectResponse{
    file_id: String,
    file_size: u64,
}

#[derive(Serialize, Deserialize, Clone,Debug)]
pub struct XmlRequestForm{
    pub external_id: String,
}

#[derive(Serialize, Deserialize, Clone,Debug)]
pub struct Xml{
    pub external_ids: Vec<String>,
    pub titles: Vec<String>,
    pub buffers: Vec<String>
}

impl Xml {
    pub fn initialize(pg_conn_pool: PgPool) -> Self{

        let channels: Vec<_> = futures::executor::block_on(
            sqlx::query!(r#"SELECT * FROM channel"#).fetch_all(&pg_conn_pool))
            .unwrap();
        let xml = Xml{
            external_ids: Vec::new(),
            titles: Vec::new(),
            //titles: vec!["Test Podcast Name".to_string()],
            buffers: Vec::new(),
        }; 
        let xml = Arc::new(RwLock::new(xml));
        for ch in channels.into_iter() {
            let external_id = ch.external_id.to_string();

            {
                let xml = xml.clone();
                let external_id = external_id.clone();
                let pg_conn_pool = pg_conn_pool.clone();
                tokio::spawn( async move {
                    let buffer = refresh_xml_buffer(&external_id.clone(), &pg_conn_pool.clone()).await.unwrap();
                    xml.write().unwrap().buffers.push(buffer);
                });
                // we don't need to abort. tokio::spawn's handle. Handles itself. No join(). 
                //std::thread::sleep(std::time::Duration::from_secs(10));
                //handle.abort();
            }

            xml.write().unwrap().external_ids.push(external_id.clone().to_string()); // temp
            xml.write().unwrap().titles.push(ch.title.clone());
        }
        
        while Arc::strong_count(&xml) > 1{
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
        return Arc::try_unwrap(xml).unwrap().into_inner().unwrap();
    } 

    pub fn get_vec_pos(&self, requested_id: &str) -> Option<usize> {
        for (i, id) in self.external_ids.iter().enumerate() {
            if requested_id == id{
                return Some(i);
            }
        }
        return None;
    }

    pub fn get_vec_pos_by_title(&self, requested_title: &str) -> Option<usize> {
        let requested_title = requested_title.to_lowercase();
        for (i, title) in self.titles.iter().enumerate() {
            if requested_title == title.to_lowercase(){
                return Some(i);
            }
        }
        return None;
    }
}

/// GET RSS feed - d
pub async fn podcast(
    /* form: web::Form<XmlRequestForm>, */
    ch_title: web::Path<String>,
    xml: web::Data<Arc<RwLock<Xml>>>
) -> HttpResponse{
    let xml = xml.read().unwrap();
    //if let Some(i) = xml.get_vec_pos(&form.external_id){
    let ch_title = ch_title.replace("-", " "); 
    if let Some(i) = xml.get_vec_pos_by_title(&ch_title){
        return HttpResponse::Ok()
            .content_type(ContentType::xml())
            .body(xml.buffers[i].clone())
    }

    return HttpResponse::BadRequest()
        .content_type(ContentType::plaintext())
        .body("invalid xml request");
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
            .content_type(ContentType::plaintext())
            .body("No channels in DB");
    }

    let mut response_ser_json = String::new();
    channels.into_iter().for_each(|c|{ 
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
            itunes_new_feed_url: c.itunes_new_feed_url,
            itunes_explicit: c.itunes_explicit,
            itunes_owner_name: c.itunes_owner_name,
            itunes_owner_email: c.itunes_owner_email,
            sy_update_period: c.sy_update_period,
            sy_update_frequency: c.sy_update_frequency,
        
        };
            
        let serialized_c = serde_json::ser::to_string(&ch).unwrap();
        response_ser_json.push_str(&serialized_c);
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
        Uuid::parse_str(&episode.into_inner().id).unwrap()
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
        enclosure_url: res.enclosure_url,
        enclosure_type: res.enclosure_type,
        enclosure_length: res.enclosure_length,
        i_link: res.i_link,
        pub_date: res.pub_date,
        itunes_subtitle: res.itunes_subtitle,
        itunes_image: res.itunes_image,
        itunes_duration: res.itunes_duration,
    };

    let mut response_ser_json = serde_json::ser::to_string(&ep).unwrap(); 
    response_ser_json = format!("{{{}}}", response_ser_json);

    return HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(response_ser_json);
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
            .content_type(ContentType::plaintext())
            .body("episode does not exist");
    }

    match sqlx::query!(r#"
        UPDATE item SET channel_id = $1, ep_number = $2, title = $3, author = $4, description = $5,
        content_encoded = $6, enclosure_url = $7, enclosure_type = $8, enclosure_length = $9, i_link = $10, 
        pub_date = $11, itunes_subtitle = $12, itunes_image = $13, itunes_duration = $14 WHERE id = $15
        "#, Uuid::parse_str(&ep.channel_id).unwrap(), ep.ep_number, ep.author, ep.category, ep.description,
        ep.content_encoded, ep.enclosure_url, ep.enclosure_type, ep.enclosure_length, ep.i_link, 
        ep.pub_date, ep.itunes_subtitle, ep.itunes_image, ep.itunes_duration,
        Uuid::parse_str(&ep.id).unwrap()
    ).execute(pg_conn_pool.get_ref()).await{
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError()
            .content_type(ContentType::plaintext())
            .body("Failed to edit DB"),
    }
}

/// POST modify channel metadata - d
pub async fn edit_channel(
    updated_ch: web::Json<Channel>,
    pg_conn_pool: &web::Data<PgPool>,
) -> HttpResponse {
    let ch = updated_ch.into_inner();
    if!(channel_exists(&ch.title, &pg_conn_pool).await) {
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
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
        ch.language, ch.last_build_date, ch.pub_date, ch.c_link, ch.itunes_new_feed_url,
        ch.itunes_explicit, ch.itunes_owner_name, 
        ch.itunes_owner_email, ch.sy_update_period,
        ch.sy_update_frequency, Uuid::parse_str(&ch.external_id).unwrap()
    ).execute(pg_conn_pool.get_ref()).await{
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError()
            .content_type(ContentType::plaintext())
            .body("Failed to edit DB"),
    }
}

/// POST multipart upload
pub async fn upload(
    payload: MultipartForm::<PodcastDataV2>, 
    active_tokens: web::Data<RwLock<ActiveTokens>>,
    pg_conn_pool: web::Data<PgPool>,
    xml: web::Data<RwLock<Xml>>,
    s3: web::Data<S3>,
) -> HttpResponse{
    let mut podcast_data = payload.podcast_data.clone();

    if !is_valid_token(&podcast_data.session_token, active_tokens).await{
        return HttpResponse::Unauthorized().finish();
    } 

    let ch = podcast_data.channel.clone();
    let ep = &mut podcast_data.item;

    if ch.external_id != ep.channel_id{
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body(format!("channel_id and episode do not match"));
    }

    ep.id = Uuid::new_v4().to_string();
    ep.enclosure_url = format!("{}/{}.mp3", &s3.full_link, &ep.id);
    ep.enclosure_type = "audio/mpeg".to_string();
    ep.enclosure_length = payload.audio.size.to_string();

    match upload_to_s3_bucket_v2(&ep.id, &payload.audio.file.path(), &s3)
        .await{
            Ok(_) => {},
            Err(e) => {
                log::info!("Error -- podcast::upload(): upload_to_s3() unsuccessful. Err: {}", e);
                return HttpResponse::InternalServerError()
                    .content_type(ContentType::plaintext())
                    .body(e);
            },
        };

    match store_to_db(&podcast_data, &pg_conn_pool).await{
        Ok(_) => {},
        Err(e) => {
            log::info!("Error -- podcast::upload(): store_to_db() unsuccessful. Err: {}", e);
            _ = delete_from_s3_bucket(&podcast_data.item.id, &s3).await.unwrap(); // fails are silent.
            return HttpResponse::InternalServerError()
                .content_type(ContentType::plaintext())
                .body(e);
        }
    }; 
    
    let xml = xml.get_ref();
    let xml_pos = match xml.read().unwrap().get_vec_pos(&ch.external_id){
        Some(pos) => pos,
        None => {
            return HttpResponse::InternalServerError()
                .content_type(ContentType::plaintext())
                .body("could not find xml buffer");
        }
    };
        
    xml.write().unwrap().buffers[xml_pos] = 
        match refresh_xml_buffer(&ch.external_id, &pg_conn_pool).await{
            Ok(b) => b,
            Err(err) => {
                let e = format!("could not refresh_xml_buffer; Err: {}", err);
                log::info!("{}", e);
                return HttpResponse::InternalServerError()
                    .content_type(ContentType::plaintext())
                    .body(e);
            }
        };

    let session_token = (&podcast_data.session_token).to_string();
    return HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body(session_token)
}

/* fn object_to_s3() */

/// POST media file, return media file id and file size
pub async fn upload_object(mut payload: Multipart, s3: web::Data<S3>) -> HttpResponse{
    let temp_dir = s3.get_ref().temp_dir.clone();
    let file_id = Uuid::new_v4().to_string();
    let temp_file = format!("{}/{}", temp_dir, file_id);

    while let Ok(Some(mut field)) = payload.try_next().await{
        let temp_file = temp_file.clone(); // trading readability for clone.
        let mut file = web::block(move|| std::fs::File::create(temp_file)) 
            .await
            .unwrap().unwrap(); //review
        while let  Some(chunk) = field.next().await{
            let data = chunk.unwrap();
            file = web::block(move|| file.write_all(&data).map(|_| file) )
                .await
                .unwrap().unwrap() // review
        }
    }

    let response = UploadObjectResponse{
        file_id,
        file_size: fs::metadata(&temp_file).unwrap().len(),
    };
   
    let response = serde_json::ser::to_string(&response).unwrap();

    return HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(response);
}

/// POST Channel/Episode - near // linode
pub async fn upload_form(
    podcast_data: web::Json<PodcastData>,
    s3: web::Data<S3>,
    active_tokens: web::Data<RwLock<ActiveTokens>>,
    pg_conn_pool: web::Data<PgPool>,
    xml: web::Data<Arc<RwLock<Xml>>>
) -> HttpResponse {

    if !is_valid_token(&podcast_data.session_token, active_tokens).await{
        return HttpResponse::Unauthorized().finish();
    } 

    let podcast_data = &mut podcast_data.into_inner();
    let ch = &podcast_data.channel;
   //  for file_id in &podcast_data.item.id { 
    let file_path = format!("{}/{}", s3.temp_dir, &podcast_data.item.id);
    if !Path::new(&file_path).exists(){
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body("at least 1 file_id does not exist");
    } 
    //}
    let ep = &mut podcast_data.item;
    let mut potential_bad_ep_uploads = String::new();

    //eps.into_iter().for_each(|ep|{
    if ch.external_id != ep.channel_id{
        potential_bad_ep_uploads
            .push_str(&format!("\n{}", ep.channel_id))
    }
    ep.enclosure_url = format!("{}/{}.mp3", &s3.full_link, &ep.id);
    ep.enclosure_type = "audio/mpeg".to_string();
    ep.enclosure_length = fs::metadata(&file_path).unwrap().len().to_string();
    //});

    //TODO: This check threw an unexpected during test. << Resolved: had a double negative above. 
    /*
    if potential_bad_ep_uploads.len() != 0 {
        return HttpResponse::BadRequest()
            .content_type(ContentType::plaintext())
            .body(format!("wrong channel_id's for episodes: \n{}", potential_bad_ep_uploads));
    }
    */

    upload_to_s3_bucket(&[&podcast_data.item.id], &s3).await.unwrap();
    store_to_db(podcast_data, &pg_conn_pool).await.unwrap();
    let ch_external_id = podcast_data.channel.external_id.clone();
    let xml_pos = match xml.read().unwrap().get_vec_pos(&ch_external_id){
        Some(pos) => pos,
        None => {
            return HttpResponse::InternalServerError()
                .content_type(ContentType::plaintext())
                .body("could not find xml buffer");
        },
    };
    let mut xml = xml.write().unwrap();
    // TODO: Can improve ; also should be updating xml vects for new podcasts. TODO -- CRITICAL
    xml.buffers[xml_pos] = refresh_xml_buffer(&ch_external_id, pg_conn_pool.get_ref()).await.unwrap();
    HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("upload complete")
}

async fn upload_to_s3_bucket_v2(ep_id: &str, path: &Path, s3: &web::Data<S3>) -> Result<(), &'static str>{
    let s3 = s3.get_ref();
    let path = match path.to_str(){
        Some(p) => p,
        None => {
            return Err("invalid unicode or non existent path.")
        },
    };

    let stream = ByteStream::from_path(path)
        .await
        .unwrap();
    
    return match s3.client.put_object()
        .bucket(&s3.bucket)
        .key(format!("{}.mp3", ep_id))
        .acl(ObjectCannedAcl::PublicRead)
        .content_type("application/mp3")
        .body(stream)
        .send()
        .await{
            Ok(_) => Ok(()),
            Err(_) => {
                let e = "Failed to upload to S3.";
                log::error!("{}", e);
                Err(e)
            }
        };
}

async fn delete_from_s3_bucket(ep_id: &str, s3: &web::Data<S3>) -> Result<(), &'static str>{
    return match s3.client.delete_object()
        .bucket(&s3.bucket)
        .key(format!("{}.mp3", ep_id))
        .send()
        .await{
            Ok(_) => Ok(()),
            Err(_) => {
                let e = "Failed to delete from S3.";
                log::error!("{}", e);
                Err(e)
            }
        };
}

// TODO: partial upload if some succeed. CRITICAL - leaves good uploads on server if all fail.
// Note impl Display for test.
/// upload to s3, failure control not implemented
async fn upload_to_s3_bucket(file_ids: &[impl Display], s3: &web::Data<S3>) -> Result<(), &'static str>{
    let s3 = s3.get_ref();
    for file_id in file_ids{
        let stream = ByteStream::from_path(&format!("{}/{}", s3.temp_dir, file_id))
            .await
            .unwrap();
        let upload_ok = match s3.client.put_object()
            .bucket(&s3.bucket)
            .key(format!("{}.mp3", file_id))
            .acl(ObjectCannedAcl::PublicRead)
            .content_type("application/mp3")
            .body(stream)
            .send()
            .await{
                Ok(_) => true,
                Err(_) => false,
            };
        if !upload_ok{
            return Err("failed to upload to S3. Error not logged");
        }
    }
    return Ok(());
}

/// check that channel exists in db and on linode. - d
async fn channel_exists(ch_title: &str, pg_conn_pool: &web::Data<PgPool>
)-> bool{
    match sqlx::query!(
        r#" SELECT id FROM channel WHERE LOWER(title) = $1 "#, ch_title.to_lowercase()
    ).fetch_optional(pg_conn_pool.get_ref())
        .await
        .unwrap(){
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
    let ep = &podcast_data.item;

    if !(channel_exists(&ch.title, &pg_conn_pool).await){
        log::info!("TRACE -----------------------------  CREATING NEW CHANNEL ");
        sqlx::query!(r#"
            INSERT INTO channel (external_id, title, category, description, managing_editor,
            generator, image_url, image_title, image_link, image_width, image_height, language,
            last_build_date, pub_date, c_link, itunes_new_feed_url, itunes_explicit, itunes_owner_name,
            itunes_owner_email, sy_update_period, sy_update_frequency)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20, $21)
            "#, Uuid::new_v4(), ch.title, ch.category, ch.description, 
            ch.managing_editor, ch.generator, ch.image_url, ch.image_title, ch.image_link, ch.image_width, 
            ch.image_height, ch.language,ch.last_build_date, ch.pub_date, ch.c_link, 
            ch.itunes_new_feed_url, ch.itunes_explicit, 
            ch.itunes_owner_name, ch.itunes_owner_email, 
            ch.sy_update_period, ch.sy_update_frequency
        ).execute(pg_conn_pool.get_ref())
        .await
        .unwrap();
    }
   
    //for ep in eps{ // messy
    sqlx::query!(r#"
        INSERT INTO item (id, channel_id, ep_number, title, author, category, description, content_encoded,
        enclosure_url, enclosure_type, enclosure_length, i_link, pub_date, itunes_subtitle, itunes_image, itunes_duration)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        "#, Uuid::parse_str(&ep.id).unwrap(), Uuid::parse_str(&ep.channel_id).unwrap(), ep.ep_number, ep.title, 
        ep.author, ep.category, ep.description, ep.content_encoded, ep.enclosure_url, ep.enclosure_type, ep.enclosure_length, 
        ep.i_link, ep.pub_date, ep.itunes_subtitle.clone(), ep.itunes_image.clone(), ep.itunes_duration.clone(),
    ).execute(pg_conn_pool.get_ref())
    .await
    .unwrap();
    //};

    return Ok(());
}

/// refresh xml with updated db data
async fn refresh_xml_buffer(
    ch_external_id: &str,
    pg_conn_pool: &PgPool,
) -> Result<String, &'static str>{
    let ch_external_id = Uuid::parse_str(ch_external_id).unwrap();

    let ch = match sqlx::query!(
        r#" SELECT * FROM channel WHERE external_id = $1 "#,   
        ch_external_id,
    ).fetch_optional(pg_conn_pool)
    .await
    .unwrap(){
        Some(ch) => {
            ch
        },       
        None => {
            return Err("couldn't find channel in DB");
        }
    };

    let channel = Channel {
        id: ch.id,
        external_id: ch.external_id.to_string(),
        title: ch.title,
        category: ch.category,
        description: ch.description,
        managing_editor: ch.managing_editor,
        generator: ch.generator,
        image_url: ch.image_url,
        image_title : ch.image_title,
        image_link: ch.image_link,
        image_width : ch.image_width,
        image_height: ch.image_height,
        language: ch.language,
        last_build_date: ch.last_build_date,
        pub_date: ch.pub_date,
        c_link: ch.c_link,
        itunes_new_feed_url: ch.itunes_new_feed_url,
        itunes_explicit: ch.itunes_explicit,
        itunes_owner_name: ch.itunes_owner_name,
        itunes_owner_email: ch.itunes_owner_email,
        sy_update_period: ch.sy_update_period,
        sy_update_frequency: ch.sy_update_frequency,
    };
    let items_res: Vec<_> = sqlx::query!(
        r#"SELECT * FROM item WHERE channel_id = $1"#,
        ch_external_id,
        ).fetch_all(pg_conn_pool)
        .await.unwrap();
       /* 
        {
            Ok(items) => items,
            Err(_) => Vec::new(),
        };
        */
    // Used to for chronological output to xml
    // Rethinking immediate need for this atm. Postgres stays chronological.
    //let mut current_ep_num = 1; 
    let mut items = Vec::<Item>::new();
    for item_res in &items_res{
        if 1 == 1 {
            let (itunes_subtitle, itunes_image, itunes_duration) = 
                if item_res.itunes_duration == "NONE" || item_res.itunes_duration.len() < 2 {
                    ("", "", "")
                } else {
                    (item_res.itunes_subtitle.as_str(), 
                     item_res.itunes_image.as_str(), 
                     item_res.itunes_duration.as_str())
            };
            items.push(Item{
                id: item_res.id.to_string(),
                channel_id: item_res.channel_id.to_string(),
                ep_number: item_res.ep_number,
                title: item_res.title.clone(),
                author: item_res.author.clone(),
                category: item_res.category.clone(),
                description: item_res.description.clone(),
                content_encoded: item_res.content_encoded.clone(),
                enclosure_url: item_res.enclosure_url.clone(),
                enclosure_type: item_res.enclosure_type.clone(),
                enclosure_length: item_res.enclosure_length.clone(),
                i_link: item_res.i_link.clone(),
                pub_date: item_res.pub_date.clone(),
                itunes_subtitle: itunes_subtitle.to_string(),
                itunes_image: itunes_image.to_string(),
                itunes_duration: itunes_duration.to_string(),
            
            });
        }
        //current_ep_num += 1;
    }  

    /*TODO: 
     1. Can have multiple itunes categories, can also nest.
     2. Complete vendor setup for rawvoice tag.
     3. drop comment tags, wellformedweb isn't a thing anymore.
    */
   let mut xml_buffer = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0"
        xmlns:content="http://purl.org/rss/1.0/modules/content/"
        xmlns:wfw="http://wellformedweb.org/CommentAPI/"
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:atom="http://www.w3.org/2005/Atom"
        xmlns:sy="http://purl.org/rss/1.0/modules/syndication/"
        xmlns:slash="http://purl.org/rss/1.0/modules/slash/"
        xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"
        xmlns:podcast="https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/1.0.md"
        xmlns:rawvoice="http://www.rawvoice.com/rawvoiceRssModule/"
        xmlns:googleplay="http://www.google.com/schemas/play-podcasts/1.0"
    >
    <channel>
        <title>{}</title>
        <managingEditor>{}</managingEditor>
        <atom:link href="{}" rel="self" type="application/rss+xml" />
        <link>{}</link>
        <description>{}</description>
        <lastBuildDate>{}</lastBuildDate>
        <language>{}</language>
        <generator>https://github.com/L19579/L19_Santigold</generator>
        <image>
            <url>{}</url>
            <title>{}</title>
            <link>{}</link>
            <width>{}</width>
            <height>{}</height>
        </image>
        <atom:link rel="hub" href="https://pubsubhubbub.appspot.com/" />
        <itunes:new-feed-url>{}</itunes:new-feed-url>
        <itunes:summary>{}</itunes:summary>
        <itunes:author>{}</itunes:author>
        <itunes:explicit>{}</itunes:explicit>
        <itunes:image>{}</itunes:image>
        <itunes:owner>
            <itunes:name>{}</itunes:name>
            <itunes:email>{}</itunes:email>
        </itunes:owner>

        <itunes:subtitle>{}</itunes:subtitle>
        <itunes:category text="{}"/>
        <googleplay:category text="{}"/>

    "#, channel.title, channel.managing_editor, channel.c_link, channel.c_link, channel.description, channel.last_build_date,
    channel.language, channel.image_url, channel.image_title, channel.image_link,
    channel.image_width, channel.image_height, channel.itunes_new_feed_url, channel.description,
    channel.itunes_owner_name, channel.itunes_explicit, channel.image_link, channel.itunes_owner_name,
    channel.itunes_owner_email, channel.description, channel.category, channel.category,
    /* channel.sy_update_period, channel.sy_update_frequency, channel.itunes_new_feed_url, "", "", "", "", "" */);

    for item in &items{
        log::info!("TRACE -- ID -------------- {}", &item.id);
    }
    loop {
        let item: Item;
        match items.pop(){
            Some(i) => item = i,
            None => break,
        }
        xml_buffer.push_str(&format!(r#"
            <item>
                <title>{}</title>
                <author>{}</author>
                <link>{}</link>
                <pubDate>{}</pubDate>
                <guid>{}</guid>
                <category><![CDATA[{}]]></category>
                <description>{}</description>
                <content:encoded>{}</content:encoded>
                <enclosure url="{}" type="{}" length="{}"/>
                <itunes:summary>{}</itunes:summary>
            </item>
        "#, item.title, item.author, item.i_link, item.pub_date, item.id, item.category, item.description, item.content_encoded
        , item.enclosure_url, item.enclosure_type, item.enclosure_length, item.description, /* item.itunes_duration */));
    }
 
    xml_buffer.push_str(
        r#"</channel>
        </rss>"#);

    return Ok(xml_buffer);
}
