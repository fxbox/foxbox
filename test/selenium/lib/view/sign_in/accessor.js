'use strict';

var Accessor = require('../accessor');


function SignInAccessor() {
  Accessor.apply(this, arguments);
}

SignInAccessor.prototype = Object.assign({

  get password() {
    // Make sure this field is not plain text
    return this.waitForElement('#signin-pwd[type="password"]');
  },

  get submitButton() {
    return this.waitForElement('#signin-button');
  }

}, Accessor.prototype);

module.exports = SignInAccessor;
