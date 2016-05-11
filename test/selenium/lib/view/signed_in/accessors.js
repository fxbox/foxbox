'use strict';

var By = require('selenium-webdriver').By;

var SELECTORS = Object.freeze({
    signOutButton: By.id('signout-button')
});

function SignedInPageAccessor(driver) {
  this.driver = driver;
}

SignedInPageAccessor.prototype = {
   get getSignOutButton() {
        return this.driver.findElement(SELECTORS.signOutButton);
   }
};

module.exports = SignedInPageAccessor;
