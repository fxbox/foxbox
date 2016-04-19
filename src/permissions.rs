extern crate users;

use users::{ Groups, Users };
use users::os::unix::GroupExt;

pub fn check_group_membership<U: Users+ Groups>(cache: &U, expected_group_names: &[&str]) {
    let mut is_one_group_found = false;
    let mut i = 0;
    let expected_group_size = expected_group_names.len();

    while !is_one_group_found && i < expected_group_size {
        let group_name = expected_group_names[i];

        if let Some(group) = cache.get_group_by_name(group_name) {
            is_one_group_found = true;
            let current_uid = cache.get_current_uid();
            let user = cache.get_user_by_uid(current_uid).unwrap();

            if !group.members().contains(&user.name().to_owned()) {
                panic!("Not a member of the {} group", group_name);
            }
        }

        i += 1;
    }

    if !is_one_group_found {
        panic!("None of the groups {:?} is defined in /etc/group. We likely don't support your \
        Linux distribution. Please file a bug.", expected_group_names);
    }
}


#[cfg(test)]
describe! membership {
    before_each {
        use users::mock::{MockUsers, User, Group};
        use users::os::unix::GroupExt;

        let mut mocked_cache = MockUsers::with_current_uid(1000);
        let mut expected_group = Group::new(101, "expected_group");
        let current_user = User::new(1000, "current_user", 101);
        mocked_cache.add_user(current_user);
        expected_group = expected_group.add_member("current_user");
        mocked_cache.add_group(expected_group);
    }

    it "should pass when user is in the expected group" {
        check_group_membership(&mocked_cache, &["expected_group"]);
    }

    // XXX Line breaks don't work with stainless
    failing(r#"None of the groups ["non_existing_group1, non_existing_group2"] is defined in /etc/group. We likely don't support your Linux distribution. Please file a bug."#)

      "should panic when no expected group exists" {
        check_group_membership(&mocked_cache, &["non_existing_group1, non_existing_group2"]);
    }

    failing("Not a member of the empty_group group")
      "should panic when you're not a member of the given group" {
        let empty_group = Group::new(102, "empty_group");
        mocked_cache.add_group(empty_group);
        check_group_membership(&mocked_cache, &["empty_group"]);
    }
}
