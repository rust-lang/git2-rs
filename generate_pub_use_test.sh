#!/bin/bash

# This script will find all structs that are marked `pub` and create a simple
# test case per struct verifying that the struct can be `use`d.
# Structs that are part of a public module (`pub mod`) are skipped.
#
# Usage:
#   ./generate_pub_use_test.sh > src/test_use_pub_structs.rs


# Get list of public modules defined using `pub mod`
pub_mods=(`grep "pub mod " src/lib.rs | sed "s|pub mod \(.*\);|\1|g"`)
# Get list of structs marked as public (through `pub struct`) and the module
# they are defined in. Use `_@_` as separator as to allow bash to properly
# split the lines, one line per array element.
# For example: `build_@_RepoBuilder`
pub_mods_and_structs=(`grep "^pub struct" src/*.rs | grep -v "src/lib.rs" | sed "s|src/\(.*\).rs:.*pub struct \([^ <]*\).*|\1_@_\2|g"`)


# Print a test module
echo ""
echo "#![allow(unused_imports)]"
# Loop over all structs
for pub_mod_struct in ${pub_mods_and_structs[*]}; do
    # Split the string into an array at the custom separator `_@_`
    pub_mod_struct_array=(`echo ${pub_mod_struct} | sed "s|_@_| |g"`)
    struct_mod=${pub_mod_struct_array[0]}
    pub_struct=${pub_mod_struct_array[1]}
    # Skip structs that are re-exported through their module by `pub mod`
    skip="False"
    for pub_mod in ${pub_mods[*]}; do
        if [[ "${pub_mod}" == "${struct_mod}" ]]; then
            # echo "SKIPPING: pub_mod: ${pub_mod}    pub_struct: ${pub_struct}"
            skip="True"
            break
        fi
    done
    if [[ "${skip}" == "True" ]]; then
        continue
    fi
    # echo "pub_mod: ${pub_mod}    pub_struct: ${pub_struct}"

    # Use lower case for the test name.
    # The test will simply attempt to `use` the struct.
    pub_struct_lower_case=`echo ${pub_struct} | tr '[:upper:]' '[:lower:]'`
    echo ""
    echo "#[test]"
    echo "fn use_${struct_mod}_${pub_struct_lower_case}() {"
    echo "    use ${pub_struct};"
    echo "}"
done
