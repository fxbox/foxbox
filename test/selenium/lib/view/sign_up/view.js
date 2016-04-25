'use strict';

var webdriver = require('selenium-webdriver');
var SetUpAccessor = require('./accessors.js');

var SuccessfulPageView = require('../successful_page/view.js');
var successfulPageView;
var isAlertPresent = false;

function SetUpView(driver) {
    this.driver = driver;
    this.accessors = new SetUpAccessor(this.driver);
    successfulPageView = new SuccessfulPageView(this.driver);
};

SetUpView.prototype = {
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

    typePassword: function(text) {
        return this.accessors.insertPassword.sendKeys(text);
    },

    confirmTypePassword: function(text) {
        return this.accessors.confirmPassword.sendKeys(text);
    },

    successLogin: function(password, confirmPassword) {
        return this.accessors.insertPassword.sendKeys(password)
        .then(() => {
            return this.accessors.confirmPassword.sendKeys(confirmPassword);
        }).then(() => {
            return this.accessors.submitButton.click();
        }).then(() => {
            return successfulPageView;
        });
    },

    successSignUpFromApp: function(password) {
        return this.accessors.insertPassword.sendKeys(password)
        .then(() => {
            return this.accessors.confirmPassword.sendKeys(password);
        }).then(() => {
            return this.accessors.submitButton.click();
        }).then(() => {
            var ServicesView = require('../services/view');
            return new ServicesView(this.driver);
        });
    },

    failureLogin: function(password, confirmPassword) {
        return this.accessors.insertPassword.sendKeys(password)
        .then(() => {
            return this.accessors.confirmPassword.sendKeys(confirmPassword);
        }).then(() => {
            return this.accessors.submitButton.click();
        }).then(() => {
            return this.alertMessage();
        });
    },

    alertMessage: function() {
        return this.driver.switchTo().alert().getText();
    },
    
    dismissAlert: function() {
       this.driver.switchTo().alert().accept();
    },

};

module.exports = SetUpView;
