'use strict';
var webdriver = require('selenium-webdriver');
var By = require('selenium-webdriver').By;

var SELECTORS = Object.freeze({
    successMessage: By.id('thank-you')
});

function SuccessfulPageAccessor(driver) {
  this.driver = driver;
};

SuccessfulPageAccessor.prototype = {
   get successMessageLocator() {
        return this.driver.findElement(SELECTORS.successMessage);
   }
};

module.exports = SuccessfulPageAccessor;
