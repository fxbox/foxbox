'use strict';

var SignedInPageView = require('./view/signed_in/view.js');
var SetUpView = require('./view/sign_up/view.js');

function SetUpWebapp(driver) {
    this.driver = driver;
}

SetUpWebapp.prototype = {
    getSignInPage : function() {
        return new SignedInPageView(this.driver);
    },

    getSetUpView : function() {
        return new SetUpView(this.driver);
    }
};

module.exports = SetUpWebapp;
