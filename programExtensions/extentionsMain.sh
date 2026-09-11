extentions_location="$bashC/programExtensions";

load_extentions () {
    local program_extentions=(
        "dotnet" \
        "git" \
        "kubernetes" \
        "nvim" \
        "pnpm" \
        "rust" \
        "terraform" \
        "yarn" \
        "python" \
        "bun" \
        "man" \
    )

	for i in "${program_extentions[@]}"
	do
		_bashc_source_file "$extentions_location/$i/${i}Main.sh" || return 1
		alias my${i}show="$extentions_location/$i/${i}Show.sh";
		alias my${i}code="code $extentions_location/$i";
	done
}

load_extentions
