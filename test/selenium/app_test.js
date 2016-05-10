'use strict';

const Suite = require('./lib/make_suite');
const HOST_URL = 'http://fxbox.github.io/app';

var suite = new Suite('Github.io webapp', HOST_URL);

suite.build((app) => {

  describe('open the web app', () => {

    var webAppMainPage;

    beforeEach(() => {
      webAppMainPage = app.getAppMainView();
    });

    it('should log in from web app', () => {
      return webAppMainPage.connectToFoxBox().then((setUpView) => {
        setUpView.successSignUpFromApp();
      });
    });
  });
});
