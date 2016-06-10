/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/// This is a client for the users email server that allows sending invitation
/// URLs to new users.

struct UsersEmailClient {
    email_server_endpoint: String,
    invitation_url_prepath: String
}

#[derive(Serialize, Debug)]
struct InvitationRequest {
    email: String,
    url: String
}

impl UsersEmailClient {
    pub fn new(email_server_endpoint: String,
               invitation_url_prepath: String) -> UsersEmailClient {
        UsersEmailClient {
            email_server_endpoint: email_server_endpoint,
            invitation_url_prepath: invitation_url_prepath
        }
    }

    pub fn send_invitation(&self, user_email: String, invitation_path: String) {
        let body = match serde_json::to_string(&InvitationRequest {
            email: user_email,
            url: format!("{}{}", self.invitation_url_prepath, invitation_path)
        }) {
            Ok(body) => body,
            Err(_) => {
                error!("Could not send invitation email.");
                return;
            }
        };

        let client = Client::new();
        let endpoint = format!("{}/v1/invitation", self.email_server_endpoint);
        client.post(&endpoint)
              .header(Connection::close())
              .body(&body)
              .send();
    }
}
