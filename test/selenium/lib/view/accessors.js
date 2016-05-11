'use strict';

var until = require('selenium-webdriver').until;

function Accessors(driver) {
  this.driver = driver;
}

Accessors.prototype = {
  waitForElement: function(locator) {
    var element = this.driver.wait(until.elementLocated(locator));
    return this.driver.wait(until.elementIsVisible(element));
  }

};

module.exports = Accessors;
