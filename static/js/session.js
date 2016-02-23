/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

'use strict';

(function(exports) {
  var SIGN_UP = '/users/setup';
  var SIGN_IN = '/users/login';

  function sessionRequest(sessionInfo, endpoint) {
    return new Promise(function(resolve, reject) {
      var xhr = new XMLHttpRequest();
      xhr.open('POST', endpoint, true);
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
      if (endpoint === SIGN_UP) {
        var body;
        try {
          body = JSON.stringify(sessionInfo);
        } catch(e) {
          return reject(e);
        }
        xhr.send(body);
      } else {
        var auth = btoa(sessionInfo.username + ':' + sessionInfo.password);
        xhr.setRequestHeader ('Authorization', 'Basic ' + auth);
        xhr.send();
      }
    });
  }

  var Session = {
    get: function() {
      return localStorage.getItem('session');
    },

    create: function(username, email, pwd) {
      if (!username || !email || !pwd) {
        return Promise.reject();
      }
      return sessionRequest({
        email: email,
        username: username,
        password: pwd
      }, SIGN_UP);
    },

    start: function(username, pwd) {
      if (!username || !pwd) {
        return Promise.reject();
      }
      return sessionRequest({
        username: username,
        password: pwd
      }, SIGN_IN);
    },

    clear: function() {
      localStorage.setItem('session', '');
    }
  };

  exports.Session = Session;
}(window));
