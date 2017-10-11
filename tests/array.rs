extern crate cmsg;
extern crate libc;

use cmsg::AsBytes;
use cmsg::AsBytesMut;
use std::mem;
use std::io::Cursor;

use cmsg::ControlMsg::*;

#[test]
fn array_rights_single() {
    let mut storage: [usize; 3] = unsafe { mem::zeroed() };

    {
        cmsg::builder(&mut Cursor::new(storage.as_bytes_mut()))
            .rights(&[4])
            .finish()
            .unwrap();
    }

    let mut iter = cmsg::iterator(storage.as_bytes());
    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 1);
            assert_eq!(fds[0], 4);
        }
        _ => panic!("Unexpected message"),
    }

    let cmsg = iter.next();
    assert!(cmsg.is_none());
}


#[test]
fn array_rights_back_to_back() {
    let mut storage: [usize; 6] = unsafe { mem::zeroed() };

    {
        cmsg::builder(&mut Cursor::new(storage.as_bytes_mut()))
            .rights(&[4])
            .rights(&[5])
            .finish()
            .unwrap();
    }

    let mut iter = cmsg::iterator(storage.as_bytes());
    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 1);
            assert_eq!(fds[0], 4);
        }
        _ => panic!("Unexpected message"),
    }

    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 1);
            assert_eq!(fds[0], 5);
        }
        _ => panic!("Unexpected message"),
    }

    let cmsg = iter.next();
    assert!(cmsg.is_none());
}

#[test]
fn array_rights_multiple() {
    let mut storage: [usize; 3] = unsafe { mem::zeroed() };

    {
        cmsg::builder(&mut Cursor::new(storage.as_bytes_mut()))
            .rights(&[4, 5])
            .finish()
            .unwrap();
    }

    let mut iter = cmsg::iterator(storage.as_bytes());
    let cmsg = iter.next();
    assert!(cmsg.is_some());
    let cmsg = cmsg.unwrap();
    match cmsg {
        Rights(fds) => {
            assert_eq!(fds.len(), 2);
            assert_eq!(fds[0], 4);
            assert_eq!(fds[1], 5);
        }
        _ => panic!("Unexpected message"),
    }

    let cmsg = iter.next();
    assert!(cmsg.is_none());
}
