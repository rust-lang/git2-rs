#!/bin/bash

set -e

function publish {
    publish_this="$1"
    crate_name="$2"
    manifest="$3"

    if [ "$publish_this" != "true" ]
    then
        echo "Skipping $crate_name, publish not requested."
        return
    fi

    # Get the version from Cargo.toml
    version=`sed -n -E 's/^version = "(.*)"/\1/p' $manifest`

    # Check crates.io if it is already published
    set +e
    output=`curl --fail --silent --head --location https://static.crates.io/crates/$crate_name/$version/download`
    res="$?"
    set -e
    case $res in
        0)
            echo "${crate_name}@${version} appears to already be published"
            return
            ;;
        22) ;;
        *)
            echo "Failed to check ${crate_name}@${version} res: $res"
            echo "$output"
            exit 1
            ;;
    esac

    cargo publish --manifest-path $manifest --no-verify

    tag="${crate_name}-${version}"
    git tag $tag
    git push origin "$tag"
}

publish $PUBLISH_LIBGIT2_SYS libgit2-sys libgit2-sys/Cargo.toml
publish $PUBLISH_GIT2 git2 Cargo.toml
publish $PUBLISH_GIT2_CURL git2-curl git2-curl/Cargo.toml
