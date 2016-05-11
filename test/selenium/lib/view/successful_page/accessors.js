'use strict';

var By = require('selenium-webdriver').By;
var Accessors = require('../accessors');

function SuccessfulPageAccessor() {
  Accessors.apply(this, arguments);
}

SuccessfulPageAccessor.prototype = Object.assign({
  get successMessageLocator() {
    return this.waitForElement(By.id('thank-you'));
  }
}, Accessors.prototype);

module.exports = SuccessfulPageAccessor;
