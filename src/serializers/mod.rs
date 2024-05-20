use hmac::Hmac;
use sha1::Sha1;

pub mod lbp;
pub mod ps3;

type HmacSha1 = Hmac<Sha1>;