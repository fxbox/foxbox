'use strict';

var By = require('selenium-webdriver').By;
var Accessor = require('../accessor');


function SignedInPageAccessor() {
  Accessor.apply(this, arguments);
}

SignedInPageAccessor.prototype = Object.assign({
  get signOutButton() {
    return this.waitForElement(By.id('signout-button'));
  }
}, Accessor.prototype);

module.exports = SignedInPageAccessor;
