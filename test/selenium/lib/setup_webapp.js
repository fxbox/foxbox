'use strict';

var SignedInPageView = require('./view/signed_in/view.js');
var SetUpView = require('./view/sign_up/view.js');
var MainView = require('./view/app_main/view.js');

function SetUpWebapp(driver) {
    this.driver = driver;
}

SetUpWebapp.prototype = {
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
