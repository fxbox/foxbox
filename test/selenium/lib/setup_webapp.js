'use strict';

var webdriver = require('selenium-webdriver');
var SignedInPageView = require('./view/signed_in/view.js');
var SetUpView = require('./view/sign_up/view.js');
var MainView = require('./view/app_main/view.js');

const driverBuilder = new webdriver.Builder().forBrowser('firefox');


function SetUpWebapp(url) {
  console.log('started driver', url);
  this.url = url;
  this.driver = this.driver = driverBuilder.build();
}

SetUpWebapp.prototype = {
    init: function() {
      return this.driver.get(this.url)
        .then(() => this.defaultView);
    },

    stop() {
      return this.driver.quit()
        .then(() => { this.driver = null; });
    },

    get defaultView() {
      return this.getSignInPage();
    },

    getSignInPage : function() {
      return new SignedInPageView(this.driver);
    },

    getSetUpView : function() {
      return new SetUpView(this.driver);
    },

    getAppMainView : function() {
      return new MainView(this.driver);
    }
};

module.exports = SetUpWebapp;
