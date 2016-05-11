'use strict';

var SuccessfulPageAccessor = require('./accessors.js');

function SuccessfulPageView(driver) {
  this.driver = driver;
  this.accessors = new SuccessfulPageAccessor(this.driver);
}

SuccessfulPageView.prototype = {
  get loginMessage() {
    return this.accessors.successMessageLocator.getText();
  }
};

module.exports = SuccessfulPageView;
