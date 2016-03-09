'use strict';

var webdriver = require('selenium-webdriver');
var SuccessfulPageAccessor = require('./accessors.js');

function SuccessfulPageView(driver) {
    this.driver = driver;
    this.accessors = new SuccessfulPageAccessor(this.driver);
};

SuccessfulPageView.prototype = {
    successLogin: function() {
        return this.accessors.successMessageLocator.getText();
    }
};

module.exports = SuccessfulPageView;
