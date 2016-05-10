'use strict';

var By = require('selenium-webdriver').By;
var Accessors = require('../accessors');

function ServicesAccessors() {
  Accessors.apply(this, arguments);
}

ServicesAccessors.prototype = Object.assign({

  get logOutButton() {
    return this.waitForElement(By.css('.user-logout-button'));
  },

}, Accessors.prototype);

module.exports = ServicesAccessors;
