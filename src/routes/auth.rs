use crate::{
    Uuid, HttpResponse, 
    web, ContentType,
    RwLock,
};

#[derive(Clone, Debug)]
pub struct AdminPassword(pub String); 

pub struct ActiveTokens(pub Vec<String>); 

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Authenticatee{
    pub username: String,
    pub password: String,
}

// strictly using admin password for now.
pub async fn generate_session_token(
    authenticatee: web::Json<Authenticatee>, 
    admin_password: web::Data<AdminPassword>,
    active_tokens: web::Data<RwLock<ActiveTokens>>,
) -> HttpResponse{
    let authenticatee = authenticatee.into_inner();
    if admin_password.get_ref().0 == authenticatee.password{
        let session_token = Uuid::new_v4().to_string();
        active_tokens.write().unwrap().0.push(session_token.clone());
        return HttpResponse::Ok()
            .content_type(ContentType::plaintext())
            .body(Uuid::new_v4().to_string());
    }
    
    return HttpResponse::Unauthorized().finish();
}

pub async fn is_valid_token(
    token: &str, 
    active_tokens: web::Data<RwLock<ActiveTokens>>) 
-> bool {
    for active_token in &active_tokens.get_ref().read().unwrap().0{
        if token == active_token{
            return true;
        }
    }
    return false;
}
