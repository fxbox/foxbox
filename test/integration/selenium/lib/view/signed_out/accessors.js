'use strict';

var webdriver = require('selenium-webdriver');
var By = require('selenium-webdriver').By;

var SELECTORS = Object.freeze({
    page: By.id('signin')
});

function SignedOutAccessor(driver) {
  this.driver = driver;
};

SignedOutAccessor.prototype = {
   get root() {
    var signinPage = this.driver.findElement(SELECTORS.page);
    return this.driver.wait(webdriver.until.elementIsVisible(signinPage));
   }
};

module.exports = SignedOutAccessor;
