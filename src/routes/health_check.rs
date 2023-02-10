use crate::{
    HttpResponse,
};

pub async fn health_check() -> HttpResponse{
    return HttpResponse::Ok().finish();
}
