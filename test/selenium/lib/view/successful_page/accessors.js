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
        return this.driver.wait(
            webdriver.until.elementLocated(SELECTORS.successMessage))
                .then((element) =>  {
                    return this.driver.wait(
                        webdriver.until.elementIsVisible(element));
                }).then(() => {
                    return this.driver.findElement(SELECTORS.successMessage);
                });
   }
};

module.exports = SuccessfulPageAccessor;
