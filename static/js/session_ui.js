/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/* global Session */

'use strict';

var SIGN_UP         = 0;
var SIGN_UP_SUCCESS = 1;
var SIGN_IN         = 2;
var SIGNED_IN       = 3;


var ELEMENTS = [{
  screen: SIGN_UP,
  selector: '#signup-pwd1'
}, {
  screen: SIGN_UP,
  selector: '#signup-pwd2'
}, {
  screen: SIGN_UP,
  selector: '#signup-button',
  event: 'click',
  listener: 'signup'
}, {
  screen: SIGN_UP_SUCCESS,
  selector: '#location'
}, {
  screen: SIGN_IN,
  selector: '#signin-pwd'
}, {
  screen: SIGN_IN,
  selector: '#signin-button',
  event: 'click',
  listener: 'signin'
}, {
  screen: SIGNED_IN,
  selector: '#signout-button',
  event: 'click',
  listener: 'signout'
}];

function getElementName(str) {
  str = str.toLowerCase().replace('#', '');
  return str.replace(/-([a-z])/g, function(g) {
    return g[1].toUpperCase();
  });
}

var SessionUI = {

  get session() {
    if (!this._session) {
      this._session = Session.get();
    }
    return this._session;
  },

  init: function() {
    SessionUI.elements = {};
    SessionUI.screens = {
      signup: document.querySelector('#signup'),
      signupSuccess: document.querySelector('#signup-success'),
      signin: document.querySelector('#signin'),
      signedin: document.querySelector('#signedin')
    };
    if (SessionUI.session === null) {
      SessionUI.show(SIGN_UP);
    } else if (SessionUI.session.length) {
      SessionUI.show(SIGNED_IN);
    } else {
      SessionUI.show(SIGN_IN);
    }
  },

  addListener: function(element, event, listener) {
    if (!element || !event || !listener) {
      return;
    }
    element.addEventListener(event, this[listener]);
  },

  removeListener: function(element, event, listener) {
    if (!element || !event || !listener) {
      return;
    }
    element.removeEventListener(event, this[listener]);
  },

  loadElements: function(screen) {
    var self = this;
    ELEMENTS.forEach(function(element) {
      var name = getElementName(element.selector);
      if (element.screen == screen) {
        try {
          self.elements[name] = document.querySelector(element.selector);
        } catch (e) {}
        if (element.event && element.listener) {
          self.addListener(self.elements[name],
                           element.event, element.listener);
        }
        return;
      }
      if (element.event && element.listener) {
        self.removeListener(self.elements[name],
                            element.event, element.listener);
      }
      self.elements[name] = null;
    });
  },

  show: function(screen) {
    if (this.currentScreen == screen) {
      return;
    }
    this.currentScreen = screen;
    this.screens.signup.hidden = (screen != SIGN_UP);
    this.screens.signupSuccess.hidden = (screen != SIGN_UP_SUCCESS);
    this.screens.signin.hidden = (screen != SIGN_IN);
    this.screens.signedin.hidden = (screen != SIGNED_IN);
    this.loadElements(screen);
  },

  showLocation: function(location) {
    SessionUI.elements.location.innerHTML = location;
  },

  signup: function() {
    var pwd = SessionUI.elements.signupPwd1.value;
    if (pwd != SessionUI.elements.signupPwd2.value) {
      window.alert('Passwords don\'t match! Please try again.');
      return;
    }

    if (pwd.length < 8) {
      window.alert('Please use a password of at least 8 characters.');
      return;
    }

    Session.create('admin', 'admin@foxbox.local', pwd).then(function() {
      SessionUI.show(SIGN_UP_SUCCESS);
      SessionUI.showLocation(window.location.href);
    }).catch(function(error) {
      window.alert('Signup error ' + error);
    });
  },

  signin: function() {
    var pwd = SessionUI.elements.signinPwd.value;
    if (!pwd || pwd.length < 8) {
      window.alert('Invalid password');
      return;
    }

    Session.start('admin', pwd).then(function() {
      SessionUI.show(SIGNED_IN);
    }).catch(function(error) {
      window.alert('Signin error ' + error);
    });
  },

  signout: function() {
    Session.clear();
    SessionUI.show(SIGN_IN);
  }
};

document.addEventListener('DOMContentLoaded', SessionUI.init);
