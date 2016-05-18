/* Any copyright is dedicated to the Public Domain.
 * http://creativecommons.org/publicdomain/zero/1.0/ */

'use strict';

var webdriver = require('selenium-webdriver'),
    assert    = require('assert');

const Suite = require('./lib/make_suite');

var suite = new Suite('Test set up UI');

suite.build((setUpWebapp) => {
  // TODO: Clean up this work around by not using the driver anywhere
  // in this file
  var driver = setUpWebapp.driver;

describe('sessions ui', function() {

  var setUpView;
  var elements;

  var shortPasswordErrorMessage
    = 'Please use a password of at least 8 characters.';
  var errorPasswordDoNotMatch = 'Passwords don\'t match! Please try again.';


  describe('Foxbox index', function() {

    it('should be titled FoxBox', function () {
      return driver.wait(webdriver.until.titleIs('FoxBox'), 5000)
        .then(function(value) {
          assert.equal(value, true);
        });
    });

    describe('signup', function() {

      beforeEach(function() {
        return suite.browserCleanUp().then(() => {
          setUpView = setUpWebapp.getSetUpView();
        });
      });

      it('should show the signup screen by default', function() {
        return setUpView.isSetUpView();
      });

      describe('failures', function() {
        afterEach(function() {
          return setUpView.dismissAlert();
        })

        it('should reject non-matching passwords', function() {
          return setUpView.failureLogin(12345678, 1234)
            .then(text => { assert.equal(text, errorPasswordDoNotMatch); });
        });

        it('should reject short passwords', function () {
          return setUpView.failureLogin(1234, 1234)
            .then(text => { assert.equal(text, shortPasswordErrorMessage); });
        });

        it('should fail if password is not set', function() {
          return setUpView.failureLogin('', '')
            .then(text => { assert.equal(text, shortPasswordErrorMessage); });
        });
      });

      describe('success', function() {

        after(() => suite.restartFromScratch());

        it('should accept matching, long-enough passwords', function() {
          return setUpView.successLogin()
            .then(successfulPageView => successfulPageView.loginMessage)
            .then(text => { assert.equal(text, 'Thank you!'); });
        });
      });
    });
  });

  describe('once registred', function() {
    var signedInView;

    before(() => {
      return setUpWebapp.init()
        .then(setUpView => setUpView.successLogin())
        .then(successfulView => successfulView.goToSignedIn())
        .then(view => signedInView = view);
    });

    describe('signedin page', function() {
      it('should sign out', function() {
        return signedInView.signOut();
      });
    });

    describe('signin page', function() {
      var signInView;

      beforeEach(function() {
        return suite.browserCleanUp()
          .then(() => {
            signInView = setUpWebapp.getSignInPage();
          });
      });

      [{
        test: 'should reject short passwords',
        pass: 'short',
        error: 'Invalid password'
      }, {
        test: 'should reject not matching passwords',
        pass: 'longEnoughButInvalid',
        error: 'Signin error Unauthorized'
      }, {
        test: 'should fail if password is not typed',
        pass: '',
        error: 'Invalid password'
      }].forEach(config => {
        it(config.test, function() {
          return signInView.failureLogin(config.pass)
            .then(alertMessage => {
              assert.equal(alertMessage, config.error);
            });
        });
      });

      it('should accept matching, long-enough passwords', function() {
        return signInView.successLogin();
      });
    });
  });
});
});
