'use strict';

var Accessor = require('../accessor');


var SELECTORS = Object.freeze({
  // Make sure these field are not plain text
  passwordField: '#signup-pwd1[type="password"]',
  confirmPasswordField: '#signup-pwd2[type="password"]',

  submit: '#signup-button',
  successMessage: '#thank-you',
  page: '#signup'
});


function SetUpAccessor() {
  Accessor.apply(this, arguments);
}

SetUpAccessor.prototype = Object.assign({
  get root() {
    return this.waitForElement(SELECTORS.page)
  },

  get isPasswordFieldPresent() {
    return this.driver.isElementPresent(SELECTORS.passwordField);
  },

  get isConfirmPasswordFieldPresent() {
    return this.driver.isElementPresent(SELECTORS.confirmPasswordField);
  },

  get isSubmitButtonPresent() {
    return this.driver.isElementPresent(SELECTORS.submit);
  },

  get insertPassword() {
    return this.waitForElement(SELECTORS.passwordField);
  },

  get confirmPassword() {
    return this.waitForElement(SELECTORS.confirmPasswordField);
  },

  get submitButton() {
    return this.waitForElement(SELECTORS.submit);
  },

  get successMessageLocator() {
    return this.waitForElement(SELECTORS.successMessage);
  }
}, Accessor.prototype);

module.exports = SetUpAccessor;
