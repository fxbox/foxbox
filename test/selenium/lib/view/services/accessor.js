'use strict';

var By = require('selenium-webdriver').By;
var Accessor = require('../accessor');

function ServicesAccessor() {
  Accessor.apply(this, arguments);
}

ServicesAccessor.prototype = Object.assign({

  get logOutButton() {
    return this.waitForElement(By.css('.user-logout-button'));
  },

}, Accessor.prototype);

module.exports = ServicesAccessor;
