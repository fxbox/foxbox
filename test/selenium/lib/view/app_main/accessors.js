'use strict';

var By = require('selenium-webdriver').By;
var Accessors = require('../accessors');


function MainAccessors() {
  Accessors.apply(this, arguments);
}

MainAccessors.prototype = Object.assign({

  get connectToFoxBoxButton() {
    return this.waitForElement(By.css('.user-login__login-button'));
  }

}, Accessors.prototype);

module.exports = MainAccessors;
