'use strict';

var webdriver = require('selenium-webdriver');
var SignedOutPageAccessor = require('./accessors.js');

function SignedOutPageView(driver) {
    this.driver = driver;
    this.accessors = new SignedOutPageAccessor(this.driver);
};

SignedOutPageView.prototype = {
    hasSignedOut: function() {
        return this.accessors.root;
    }
};

module.exports = SignedOutPageView;
