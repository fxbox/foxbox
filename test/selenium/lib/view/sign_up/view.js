'use strict';

var View = require('../view');
var SetUpAccessor = require('./accessors.js');


function SetUpView() {
  [].push.call(arguments, SetUpAccessor);
  View.apply(this, arguments);
}

SetUpView.prototype = Object.assign({
    isSetUpView: function() {
        return this.accessors.root;
    },

    passwordField: function() {
        return this.accessors.isPasswordFieldPresent;
    },

    passwordConfirmField: function() {
        return this.accessors.isConfirmPasswordFieldPresent;
    },

    submitButtonPresent: function() {
        return this.accessors.isSubmitButtonPresent;
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

      return this.accessors.insertPassword.sendKeys(password)
        .then(() => this.accessors.confirmPassword.sendKeys(confirmPassword))
        .then(() => this.accessors.submitButton.click());
    },

    alertMessage: function() {
        return this.driver.switchTo().alert().getText();
    },

    dismissAlert: function() {
       return this.driver.switchTo().alert().accept();
    },

}, View.prototype);

module.exports = SetUpView;
