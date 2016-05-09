'use strict';

var foxboxManager = require('./foxboxHelperSelenium.js');

var testPrepperSelenium = (function() {

  function beforeTest(done) {
    console.log('test started');
    foxboxManager.fullOptionStart(done);
  }

  function makeSuite(desc, test) {
    describe(desc, function () {
      this.timeout(30000);
      before(beforeTest);
      test();
      after(function() {
        foxboxManager.killFoxBox();
        return foxboxManager.cleanData();
      });
    });
  }

  return { makeSuite, foxboxManager};
})();

module.exports = testPrepperSelenium;
