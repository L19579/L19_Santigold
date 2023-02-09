use crate::{
    HttpResponse,
};

pub async fn health_check() -> {
    return HttpResponse::Ok().finish();
}
