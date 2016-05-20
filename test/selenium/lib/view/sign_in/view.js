'use strict';

var View = require('../view');


function SignInView() {
  View.apply(this, arguments);
}

SignInView.prototype = Object.assign({
  successLogin(password) {
    password = password !== undefined ? password : 12345678;
    return this._submitPassword(password)
    .then(() => this.instanciateNextView('signed_in'));
  },

  failureLogin(password) {
    return this._submitPassword(password)
    .then(() => this.alertMessage());
  },

  _submitPassword(password) {
    return this.accessor.passwordField.sendKeys(password)
    .then(() => this.accessor.submitButton.click());
  },

  alertMessage() {
    return this.driver.switchTo().alert().getText();
  },

  dismissAlert() {
    return this.driver.switchTo().alert().accept();
  },
}, View.prototype);

module.exports = SignInView;
