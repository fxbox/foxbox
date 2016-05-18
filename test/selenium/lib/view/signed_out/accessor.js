'use strict';

var By = require('selenium-webdriver').By;
var Accessor = require('../accessor');


function SignedOutAccessor() {
  Accessor.apply(this, arguments);
}

SignedOutAccessor.prototype = Object.assign({
  get root() {
    return this.waitForElement(By.id('signin'));
  }
}, Accessor.prototype);

module.exports = SignedOutAccessor;
