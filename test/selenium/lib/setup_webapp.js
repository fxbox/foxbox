'use strict';

var SignedInPageView = require('./view/signed_in/view.js');

function SetUpWebapp(driver) {
    this.driver = driver;
}

SetUpWebapp.prototype = {
    getSignInPage : function() {
        return new SignedInPageView(this.driver);
    }
};

module.exports = SetUpWebapp;
