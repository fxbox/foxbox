'use strict';

var webdriver = require('selenium-webdriver');
var SuccessfulPageAccessor = require('./accessors.js');

function SuccessfulPageView(driver) {
    this.driver = driver;
    this.accessors = new SuccessfulPageAccessor(this.driver);
};

SuccessfulPageView.prototype = {
    loginMessage: function() {
        return this.accessors.successMessageLocator.then((element) => {
            return element.getText();
        });
    }
};

module.exports = SuccessfulPageView;
