'use strict';

var By = require('selenium-webdriver').By;
var Accessor = require('../accessor');

function SuccessfulPageAccessor() {
  Accessor.apply(this, arguments);
}

SuccessfulPageAccessor.prototype = Object.assign({
  get successMessageLocator() {
    return this.waitForElement(By.id('thank-you'));
  }
}, Accessor.prototype);

module.exports = SuccessfulPageAccessor;
