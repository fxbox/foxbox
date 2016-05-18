'use strict';

var View = require('../view');


function SignInView() {
  View.apply(this, arguments);
}

SignInView.prototype = Object.assign({
  successLogin: function(password) {
    password = password !== undefined ? password : 12345678;
    return this._submitPassword(password)
    .then(() => this.instanciateNextView('signed_in'));
  },

  failureLogin: function(password) {
    return this._submitPassword(password)
    .then(() => this.alertMessage());
  },

  _submitPassword: function(password) {
    return this.accessor.password.sendKeys(password)
    .then(() => this.accessor.submitButton.click());
  },

  alertMessage: function() {
    return this.driver.switchTo().alert().getText();
  },

  dismissAlert: function() {
    return this.driver.switchTo().alert().accept();
  },
}, View.prototype);

module.exports = SignInView;
