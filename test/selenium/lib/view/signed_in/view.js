'use strict';

var SignedInPageAccessor = require('./accessors.js');
var SignedOutPageView = require('../signed_out/view.js');

var signedOutPageView;

function SignedInPageView(driver) {
    this.driver = driver;
    this.accessors = new SignedInPageAccessor(this.driver);
    signedOutPageView = new SignedOutPageView(this.driver);
}

SignedInPageView.prototype = {
    signOut: function() {
    	return this.accessors.getSignOutButton.click().then(function() {
            return signedOutPageView.hasSignedOut();
        });
    }
};

module.exports = SignedInPageView;
