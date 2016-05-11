'use strict';

function View(driver, Accessors) {
  this.driver = driver;
  this.accessors = new Accessors(driver);
}

module.exports = View;
