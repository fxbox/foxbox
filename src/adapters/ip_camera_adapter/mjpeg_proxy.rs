extern crate hyper;

use std::io;
use std::io::{Read, Write};

use iron::Response;
use iron::headers::{AccessControlAllowOrigin, CacheControl, CacheDirective, ContentType, Pragma};
use iron::mime::{Attr, Mime, SubLevel, TopLevel, Value};
use iron::response::{WriteBody, ResponseBody};
use iron::status::Status;

use self::hyper::header::{Authorization, Basic, Connection};

const MIME_SUB_LEVEL: &'static str = "x-mixed-replace";
const MIME_MULTIPART_BOUNDARY: &'static str = "video boundary--";

struct MJPEGProxyStreamer {
    url: String,
    username: String,
    password: Option<String>,
}

unsafe impl Send for MJPEGProxyStreamer {}

impl WriteBody for MJPEGProxyStreamer {
    fn write_body(&mut self, res: &mut ResponseBody) -> io::Result<()> {
        let client = hyper::Client::new();

        let authorizaton_header = Authorization(
            Basic {
                username: self.username.clone(),
                password: self.password.clone()
            }
        );

        let mjpeg_request_result =  client.get(&self.url)
            .header(authorizaton_header)
            .header(Connection::keep_alive())
            .send();

        let mut mjpeg_stream = match mjpeg_request_result {
            Ok(res) => res,
            Err(err) => {
                let error_string = format!("Failed to get MJPEG stream for {}: {}", self.url, err);
                return Err(io::Error::new(io::ErrorKind::Other, error_string));
            }
        };

        if mjpeg_stream.status != Status::Ok {
            let error_string = format!("Failed to get MJPEG stream for {}: {}",
                                       self.url, mjpeg_stream.status);
            return Err(io::Error::new(io::ErrorKind::Other, error_string));
        }

        // Let's read data with 5120-byte chunks and flush it to the client.
        let mut buffer: [u8;5120] = [0;5120];
        loop {
            if let Err(err) = mjpeg_stream.read_exact(&mut buffer) {
                debug!("Can't read MJPEG stream: {:?}", err);
                break;
            } else if let Err(err) = res.write(&buffer) {
                debug!("Can't write MJPEG stream to the client: {:?}", err);
                break;
            } else if let Err(err) = res.flush() {
                debug!("Can't flush MJPEG stream to the client: {:?}", err);
                break;
            }
        }

        Ok(())
    }
}

pub struct MJPEGProxy;

impl MJPEGProxy {
    pub fn create_response(url: String, username: String, password: Option<String>) -> Response {
        let streamer: Box<WriteBody + Send> = Box::new(
            MJPEGProxyStreamer {
                url: url,
                username: username,
                password: password
            }
        );

        let mut response = Response::with(streamer);

        response.status = Some(Status::Ok);
        response.headers.set(AccessControlAllowOrigin::Any);

        // Live MJPEG stream should not be cached.
        response.headers.set(CacheControl(vec![CacheDirective::NoCache]));
        response.headers.set(Pragma::NoCache);

        // Ideally we should use headers returned by streamer, but simplifying for now.
        let mime = Mime(TopLevel::Multipart, SubLevel::Ext(MIME_SUB_LEVEL.to_owned()),
                        vec![(Attr::Boundary, Value::Ext(MIME_MULTIPART_BOUNDARY.to_owned()))]);
        response.headers.set(ContentType(mime));

        response
    }
}
