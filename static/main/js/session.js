/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

'use strict';

(function(exports) {

  function sessionRequest(sessionInfo) {
    return new Promise(function(resolve, reject) {
      var xhr = new XMLHttpRequest();
      xhr.open('POST', '/users/login', true);
      xhr.onload = function() {
        var response;
        try {
          response = JSON.parse(xhr.responseText);
        } catch(e) {
          return reject('Invalid response');
        }
        if (xhr.status != 201) {
          return reject(response.error);
        }
        var token = response.session_token;
        if (!token) {
          return reject('Missing token');
        }
        localStorage.setItem('session', token);
        resolve();
      };
      // See https://github.com/fxbox/users/blob/master/doc/API.md#post-setup
      xhr.setRequestHeader('Content-Type', 'application/json');
      var auth = btoa(sessionInfo.username + ':' + sessionInfo.password);
      xhr.setRequestHeader ('Authorization', 'Basic ' + auth);
      xhr.send();
    });
  }

  var Session = {
    get: function() {
      return localStorage.getItem('session');
    },

    start: function(username, pwd) {
      if (!username || !pwd) {
        return Promise.reject();
      }
      return sessionRequest({
        username: username,
        password: pwd
      });
    },

    clear: function() {
      localStorage.clear('session');
    }
  };

  exports.Session = Session;
}(window));
