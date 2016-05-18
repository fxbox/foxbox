'use strict';

var until = require('selenium-webdriver').until;

function Accessor(driver) {
  this.driver = driver;
}

Accessor.prototype = {
  waitForElement: function(locator) {
    var element = this.driver.wait(until.elementLocated(locator));
    return this.driver.wait(until.elementIsVisible(element));
  }

};

module.exports = Accessor;
