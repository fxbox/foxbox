'use strict';

var View = require('../view');


function SetUpView() {
  View.apply(this, arguments);
}

SetUpView.prototype = Object.assign({
    isSetUpView: function() {
        return this.accessor.root;
    },

    passwordField: function() {
        return this.accessor.isPasswordFieldPresent;
    },

    passwordConfirmField: function() {
        return this.accessor.isConfirmPasswordFieldPresent;
    },

    submitButtonPresent: function() {
        return this.accessor.isSubmitButtonPresent;
    },

    successLogin: function(password) {
      return this._submitPassword(password)
      .then(() => this.instanciateNextView('successful_page'));
    },

    successSignUpFromApp: function(password) {
      return this._submitPassword(password)
      .then(() => this.instanciateNextView('services'));
    },

    failureLogin: function(password, confirmPassword) {
      return this._submitPassword(password, confirmPassword)
        .then(() => this.alertMessage());
    },

    _submitPassword: function(password, confirmPassword) {
      password = password !== undefined ? password : 12345678;
      confirmPassword = confirmPassword !== undefined ?
        confirmPassword : password;

      return this.accessor.insertPassword.sendKeys(password)
        .then(() => this.accessor.confirmPassword.sendKeys(confirmPassword))
        .then(() => this.accessor.submitButton.click());
    },

    alertMessage: function() {
        return this.driver.switchTo().alert().getText();
    },

    dismissAlert: function() {
       return this.driver.switchTo().alert().accept();
    },

}, View.prototype);

module.exports = SetUpView;
