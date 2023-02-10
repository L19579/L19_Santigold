use {
    crate::{
        web,
        HttpResponse,
        ContentType,
    },
    serde::{
        Serialize, Deserialize,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ExampleJson{
    pub channel: Channel,
    pub item: Vec<Item>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Channel{
    pub title: String,
    pub link: String,
    pub description: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Item{
    pub title: String,
    pub link: String,
    pub description: String,
}

/// pulse check - good
pub async fn health_check() -> HttpResponse{
    log::info!("/health_check is reachable");
    return HttpResponse::Ok()
        .content_type(ContentType::plaintext())
        .body("This is a body provided by /health_check GET response.");
}

/// pulse check - return XML - good
pub async fn health_check_xml() -> HttpResponse{
    let body = r#"<?xml version="1.0" encoding="UTF-8" ?>
            <rss version="2.0">

            <channel>
              <title>W3Schools Home Page</title>
              <link>https://www.w3schools.com</link>
              <description>Free web building tutorials</description>
              <item>
                <title>RSS Tutorial</title>
                <link>https://www.w3schools.com/xml/xml_rss.asp</link>
                <description>New RSS tutorial on W3Schools</description>
              </item>
              <item>
                <title>XML Tutorial</title>
                <link>https://www.w3schools.com/xml</link>
                <description>New XML tutorial on W3Schools</description>
              </item>
            </channel>

            </rss>"#;
    return HttpResponse::Ok()
    .content_type(ContentType::xml())
    .body(body);
}

/// pulse check - convert JSON data to xml, return XML - good.
pub fn json_to_xml(json_str: &str) -> String{
    let json_deserialize: ExampleJson = serde_json::from_str(json_str).unwrap();

    let mut converted_body = format!(r#"<?xml version="1.0" encoding="UTF-8" ?>
        <rss version="2.0">
        <channel>
            <title>
                {}
            </title>
            <link>
                {}
            </link>
            <description>
                {}
            </description>
        </channel>"#, json_deserialize.channel.title, json_deserialize.channel.link, 
    json_deserialize.channel.description);
    
    for item in json_deserialize.item{
        let item_xml = format!(r#"
        <item>
            <title>
                {}
            </title>
            <link>
                {}
            </link>
            <description>
                {}
            </description>
        </item>"#, item.title, item.link, item.description);

        converted_body.push_str(&item_xml);
    }
    converted_body.push_str(&format!("</rss>"));

    return converted_body;
}

/// pulse check - receive JSON, convert and return XML - good.
pub async fn health_check_xml_extended() -> HttpResponse{
    let example_input_json = r#"{
            "channel": {
                "title": "W3schools Home Page",
                "link": "http://thisisapretendlink.com",
                "description": "RSS tutorial for W3School"
            },
            "item": [
                {
                "title": "Podcast Title 1",
                "link": "http://podcastlink.cl",
                "description": " Podcast 1 description."
                },
                {
                "title": "Podcast Title 2",
                "link": "http://podcastlink.cl",
                "description": " Podcast 2 description."
                },
                {
                "title": "Podcast Title 2",
                "link": "http://podcastlink.cl",
                "description": " Podcast 2 description."
                },
                {
                "title": "Podcast Title 2",
                "link": "http://podcastlink.cl",
                "description": " Podcast 2 description."
                },
                {
                "title": "Podcast Title 2",
                "link": "http://podcastlink.cl",
                "description": " Podcast 2 description."
                }
            ]
    }"#;

    let converted_body = json_to_xml(example_input_json);

    log::info!("{}", converted_body);
    return HttpResponse::Ok()
    .content_type(ContentType::xml())
    .body(converted_body)
}

pub async fn health_check_xml_extended_post(form: web::Json<ExampleJson>) -> HttpResponse{
    let example_input_json = form.into_inner();
    let converted_body = json_to_xml(&serde_json::ser::to_string(&example_input_json).unwrap());

    log::info!("{}", converted_body);
    return HttpResponse::Ok()
    .content_type(ContentType::xml())
    .body(converted_body)
}
