'use strict';

const makeSuite = require('./lib/make_suite');
const HOST_URL = 'http://fxbox.github.io/app';


makeSuite('Github.io webapp', HOST_URL, (app) => {

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
