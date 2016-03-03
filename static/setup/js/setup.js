/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

'use strict';

var SetupUI = {
  init: function() {
    SetupUI.elements = {
      signupPwd1: document.querySelector('#signup-pwd1'),
      signupPwd2: document.querySelector('#signup-pwd2'),
      signupButton: document.querySelector('#signup-button'),
      location: document.querySelector('#location'),
    };
    SetupUI.screens = {
      signup: document.querySelector('#signup'),
      signupSuccess: document.querySelector('#signup-success')
    };
    SetupUI.elements.signupButton.addEventListener('click', SetupUI.signup);
  },

  signup: function() {
    var pwd = SetupUI.elements.signupPwd1.value;
    if (pwd != SetupUI.elements.signupPwd2.value) {
      window.alert('Passwords don\'t match! Please try again.');
      return;
    }

    if (pwd.length < 8) {
      window.alert('Please use a password of at least 8 characters.');
      return;
    }

    var xhr = new XMLHttpRequest();
    xhr.open('POST', '/users/setup', true);
    xhr.onload = function() {
      var response;
      try {
        response = JSON.parse(xhr.responseText);
      } catch(e) {
        window.alert('Invalid response');
        return;
      }
      if (xhr.status != 201) {
        window.alert(response.error);
        return;
      }
      var token = response.session_token;
      if (!token) {
        window.alert('Missing token');
        return
      }
      localStorage.setItem('session', token);
      SetupUI.elements.location.innerHTML = window.location.href;
      SetupUI.screens.signupSuccess.hidden = false;
      SetupUI.screens.signup.hidden = true;
    };
    // See https://github.com/fxbox/users/blob/master/doc/API.md#post-setup
    xhr.setRequestHeader('Content-Type', 'application/json');
    var body;
    try {
      body = JSON.stringify({
        username: 'admin',
        email: 'admin@foxbox.local',
        password: pwd
      });
    } catch(e) {
      return reject(e);
    }
    xhr.send(body);
  }
};

document.addEventListener('DOMContentLoaded', SetupUI.init);
