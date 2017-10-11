extern crate cmsg;
extern crate libc;

use cmsg::ControlMsg::*;

#[test]
fn vec_rights_single() {
    let mut storage: Vec<u8> = Vec::with_capacity(1024);

    cmsg::builder(&mut storage)
        .rights(&[4])
        .finish()
        .unwrap();

    let mut iter = cmsg::iterator(storage.as_ref());
    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 1);
            assert_eq!(fds[0], 4);
        }
        _ => panic!("Unexpeced message")
    }

    let cmsg = iter.next();
    assert!(cmsg.is_none());
}

#[test]
fn vec_rights_back_to_back() {
    let mut storage: Vec<u8> = Vec::with_capacity(1024);

    cmsg::builder(&mut storage)
        .rights(&[4])
        .rights(&[5])
        .finish()
        .unwrap();

    let mut iter = cmsg::iterator(storage.as_ref());
    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 1);
            assert_eq!(fds[0], 4);
        }
        _ => panic!("Unexpeced message")
    }

    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 1);
            assert_eq!(fds[0], 5);
        }
        _ => panic!("Unexpeced message")
    }

    let cmsg = iter.next();
    assert!(cmsg.is_none());
}


#[test]
fn vec_rights_multiple() {
    let mut storage: Vec<u8> = Vec::with_capacity(1024);

    cmsg::builder(&mut storage)
        .rights(&[4, 5])
        .finish()
        .unwrap();

    let mut iter = cmsg::iterator(storage.as_ref());
    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 2);
            assert_eq!(fds[0], 4);
            assert_eq!(fds[1], 5);
        }
        _ => panic!("Unexpeced message")
    }

    let cmsg = iter.next();
    assert!(cmsg.is_none());
}
