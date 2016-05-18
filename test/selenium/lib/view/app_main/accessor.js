'use strict';

var By = require('selenium-webdriver').By;
var Accessor = require('../accessor');


function MainAccessor() {
  Accessor.apply(this, arguments);
}

MainAccessor.prototype = Object.assign({

  get connectToFoxBoxButton() {
    return this.waitForElement(By.css('.user-login__login-button'));
  }

}, Accessor.prototype);

module.exports = MainAccessor;
