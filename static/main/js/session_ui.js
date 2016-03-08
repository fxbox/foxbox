/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/* global Session */
/* global URLSearchParams */

'use strict';

var SIGN_IN         = 2;
var SIGNED_IN       = 3;


var ELEMENTS = [{
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
      signin: document.querySelector('#signin'),
      signedin: document.querySelector('#signedin')
    };

    var searchParams =
      new URLSearchParams(window.location.search.substring(1));

    if (searchParams.has('redirect_url')) {
      SessionUI.redirect = searchParams.get('redirect_url');
    }

    if (SessionUI.session === null) {
      SessionUI.show(SIGN_IN);
    } else if (SessionUI.session.length) {
      SessionUI.show(SIGNED_IN);
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
    this.screens.signin.hidden = (screen != SIGN_IN);
    this.screens.signedin.hidden = (screen != SIGNED_IN);
    this.loadElements(screen);
  },

  signin: function() {
    var pwd = SessionUI.elements.signinPwd.value;
    if (!pwd || pwd.length < 8) {
      window.alert('Invalid password');
      return;
    }

    Session.start('admin', pwd, SessionUI.redirect === undefined).then(
      function(token) {
      if (SessionUI.redirect) {
        window.location.replace(SessionUI.redirect + '?session_token=' + token);
      } else {
        SessionUI.show(SIGNED_IN);
      }
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
